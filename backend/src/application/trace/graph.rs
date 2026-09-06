//! 追踪图派生纯函数。
//!
//! 所有派生视图（追溯链 / 影响面 / 覆盖 / 追溯矩阵）都是**读取期计算**，不落库
//! （对齐 knowledge-base T4/P3）。追踪图由大系统灌入，**不能假设无环**，故遍历
//! 一律用 visited 集合防环/防栈溢出（对齐 pitfalls P4 的方向教训：别默认 DAG）。
//!
//! 方向约定：link 存储方向 `source → target`（source=下级/派生方，target=上级/
//! 被满足方）。因此：
//! - `Up`   = 沿存储方向走（source→target）：某节点 trace 到的更上级对象；
//! - `Down` = 反向走（target→source）：某节点被哪些下级 trace/派生（= 影响面/impact）。

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use uuid::Uuid;

use crate::domain::trace::{LinkType, TraceNode, TraceNodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

fn allowed_fn(allowed: Option<&[LinkType]>) -> impl Fn(LinkType) -> bool + '_ {
    move |t| allowed.map_or(true, |list| list.contains(&t))
}

/// 邻接表（按方向）。返回 map: 起点 -> 可达终点集合。
fn adjacency(
    links: &[(Uuid, Uuid, LinkType)],
    dir: Direction,
    allowed: Option<&[LinkType]>,
) -> HashMap<Uuid, Vec<Uuid>> {
    let ok = allowed_fn(allowed);
    let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for &(s, t, lt) in links {
        if !ok(lt) {
            continue;
        }
        let (from, to) = match dir {
            Direction::Up => (s, t),
            Direction::Down => (t, s),
        };
        map.entry(from).or_default().push(to);
    }
    map
}

/// 从 roots 出发沿 direction 的闭包（不含 roots 本身；防环）。
pub fn closure(
    links: &[(Uuid, Uuid, LinkType)],
    roots: &[Uuid],
    dir: Direction,
    allowed: Option<&[LinkType]>,
) -> HashSet<Uuid> {
    let adj = adjacency(links, dir, allowed);
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut queue: VecDeque<Uuid> = roots.iter().copied().collect();
    while let Some(n) = queue.pop_front() {
        if !visited.insert(n) {
            continue;
        }
        if let Some(nexts) = adj.get(&n) {
            for &m in nexts {
                if !visited.contains(&m) {
                    queue.push_back(m);
                }
            }
        }
    }
    visited.retain(|n| !roots.contains(n));
    visited
}

/// 直接 trace 进 target 的 source 集合（allowed 限定 link_type）。用于追溯矩阵。
pub fn direct_sources(
    links: &[(Uuid, Uuid, LinkType)],
    target: Uuid,
    allowed: Option<&[LinkType]>,
) -> Vec<(Uuid, LinkType)> {
    let ok = allowed_fn(allowed);
    links
        .iter()
        .filter(|&&(_, t, lt)| t == target && ok(lt))
        .map(|&(s, _, lt)| (s, lt))
        .collect()
}

/// 覆盖分析：没有任何 allowed 类型的边 trace 进来的 `SoftwareRequirement`。
/// allowed 缺省用派生/验证语义（derives_from / satisfies / verifies）。
pub fn uncovered(nodes: &[TraceNode], links: &[(Uuid, Uuid, LinkType)]) -> Vec<Uuid> {
    let allowed: &[LinkType] = &[LinkType::DerivesFrom, LinkType::Satisfies, LinkType::Verifies];
    let incoming: HashSet<Uuid> = links
        .iter()
        .filter(|&&(_, _, lt)| allowed.contains(&lt))
        .map(|&(_, t, _)| t)
        .collect();
    nodes
        .iter()
        .filter(|n| n.kind == TraceNodeKind::SoftwareRequirement && !incoming.contains(&n.id))
        .map(|n| n.id)
        .collect()
}

// ---------------------------------------------------------------------------
// 自然序 / 需求锚定排序（「整理」按钮与列内默认顺序共用同一规则）
// ---------------------------------------------------------------------------

/// 数字感知的字符串比较：`TC-2` < `TC-10`（而非字典序的 `TC-10` < `TC-2`）。
/// 数字段按数值比较，非数字段按字典序；数字排在字母前。
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    fn tokens(s: &str) -> Vec<NT> {
        let cs: Vec<char> = s.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < cs.len() {
            if cs[i].is_ascii_digit() {
                let mut j = i;
                while j < cs.len() && cs[j].is_ascii_digit() {
                    j += 1;
                }
                let v: String = cs[i..j].iter().collect();
                out.push(NT::Num(v.parse::<u64>().unwrap_or(0)));
                i = j;
            } else {
                let mut j = i;
                while j < cs.len() && !cs[j].is_ascii_digit() {
                    j += 1;
                }
                out.push(NT::Str(cs[i..j].iter().collect()));
                i = j;
            }
        }
        out
    }
    enum NT {
        Num(u64),
        Str(String),
    }
    let (ta, tb) = (tokens(a), tokens(b));
    for k in 0..ta.len().min(tb.len()) {
        let o = match (&ta[k], &tb[k]) {
            (NT::Num(x), NT::Num(y)) => x.cmp(y),
            (NT::Num(_), NT::Str(_)) => Ordering::Less,
            (NT::Str(_), NT::Num(_)) => Ordering::Greater,
            (NT::Str(x), NT::Str(y)) => x.cmp(y),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    ta.len().cmp(&tb.len())
}

/// 计算全图规范顺序（返回 `(node_id, sort_key)`，每阶段内 0..n-1）。
///
/// 规则（用户定案）：排序以**需求顺序**为基准——
/// 1. 需求按其当前 `(sort_key, 自然序 ref)` 排成需求序，得到 rank；
/// 2. 每个非需求节点向上闭包（它 trace/回答到的所有更上级）里，取 **rank 最小**
///    （最靠前）的那个需求作为锚点；无任何需求可达的节点排最后；
/// 3. 每阶段内按 `(锚点需求 rank, 自然序 ref)` 排序，紧凑重排 sort_key。
///
/// 用途：`POST /api/trace/nodes/arrange`（整理按钮）落库；也是列内默认顺序的来源。
pub fn requirement_anchored_order(
    nodes: &[TraceNode],
    links: &[(Uuid, Uuid, LinkType)],
) -> Vec<(Uuid, i32)> {
    use TraceNodeKind::{SoftwareRequirement, TestCase, TestItem, TestRecord, TestReport};
    const KIND_ORDER: [TraceNodeKind; 5] =
        [TestReport, TestRecord, TestCase, TestItem, SoftwareRequirement];

    fn sr_sort_key(a: &TraceNode, b: &TraceNode) -> Ordering {
        a.sort_key
            .cmp(&b.sort_key)
            .then_with(|| natural_cmp(&a.external_ref, &b.external_ref))
            .then_with(|| a.external_source.cmp(&b.external_source))
            .then_with(|| a.id.cmp(&b.id))
    }

    // 需求排成需求序 → rank（越靠前越小）
    let mut srs: Vec<&TraceNode> = nodes
        .iter()
        .filter(|n| n.kind == SoftwareRequirement)
        .collect();
    srs.sort_by(|a, b| sr_sort_key(a, b));
    let rank: HashMap<Uuid, usize> = srs
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect();

    // 每节点 → 向上可达需求中的最小 rank（无则 usize::MAX → 排最后）
    let mut anchor: HashMap<Uuid, usize> = HashMap::new();
    for n in nodes {
        if n.kind == SoftwareRequirement {
            continue;
        }
        let reach = closure(links, &[n.id], Direction::Up, None);
        let mut best = usize::MAX;
        for rid in reach {
            if let Some(&r) = rank.get(&rid) {
                best = best.min(r);
            }
        }
        anchor.insert(n.id, best);
    }
    // 顺手把每个需求的锚记为自己 rank（需求列自身参与排序时无需走 anchor）
    for (id, r) in &rank {
        anchor.insert(*id, *r);
    }

    let mut by_kind: HashMap<TraceNodeKind, Vec<&TraceNode>> = HashMap::new();
    for n in nodes {
        by_kind.entry(n.kind).or_default().push(n);
    }

    let mut out = Vec::with_capacity(nodes.len());
    for kind in KIND_ORDER {
        let mut arr = by_kind.remove(&kind).unwrap_or_default();
        if kind == SoftwareRequirement {
            arr.sort_by(|a, b| sr_sort_key(a, b));
        } else {
            arr.sort_by(|a, b| {
                anchor
                    .get(&a.id)
                    .unwrap_or(&usize::MAX)
                    .cmp(anchor.get(&b.id).unwrap_or(&usize::MAX))
                    .then_with(|| natural_cmp(&a.external_ref, &b.external_ref))
                    .then_with(|| a.external_source.cmp(&b.external_source))
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        for (i, n) in arr.into_iter().enumerate() {
            out.push((n.id, i as i32));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(s: u32, t: u32, lt: LinkType) -> (Uuid, Uuid, LinkType) {
        (u32_to_uuid(s), u32_to_uuid(t), lt)
    }
    fn u32_to_uuid(v: u32) -> Uuid {
        Uuid::from_u128(v as u128)
    }

    #[test]
    fn closure_up_and_down_are_mirrors() {
        // 1 --derives_from--> 2 --> ... nothing; simple: 1 -> 2 -> 3
        let links = [e(1, 2, LinkType::DerivesFrom), e(2, 3, LinkType::DerivesFrom)];
        let roots = [u32_to_uuid(1)];
        let up = closure(&links, &roots, Direction::Up, None);
        assert!(up.contains(&u32_to_uuid(2)) && up.contains(&u32_to_uuid(3)));

        let roots3 = [u32_to_uuid(3)];
        let down = closure(&links, &roots3, Direction::Down, None);
        assert!(down.contains(&u32_to_uuid(2)) && down.contains(&u32_to_uuid(1)));
    }

    #[test]
    fn closure_handles_cycle_without_infinite_loop() {
        let links = [e(1, 2, LinkType::DerivesFrom), e(2, 1, LinkType::DerivesFrom)];
        let roots = [u32_to_uuid(1)];
        let down = closure(&links, &roots, Direction::Down, None);
        // cycle: 1's down includes 2 (which points to 1); 1 excluded from result
        assert!(down.contains(&u32_to_uuid(2)));
        let up = closure(&links, &roots, Direction::Up, None);
        assert!(up.contains(&u32_to_uuid(2)));
    }

    #[test]
    fn allowed_filter_excludes_reference_like_links() {
        let links = [
            e(1, 2, LinkType::DerivesFrom),
            e(1, 3, LinkType::PartOf), // 分解不驱动追溯语义
        ];
        let roots = [u32_to_uuid(1)];
        let up = closure(&links, &roots, Direction::Up, Some(&[LinkType::DerivesFrom]));
        assert!(up.contains(&u32_to_uuid(2)));
        assert!(!up.contains(&u32_to_uuid(3)));
    }

    #[test]
    fn natural_cmp_is_number_aware() {
        assert_eq!(natural_cmp("TC-2", "TC-10"), Ordering::Less);
        assert_eq!(natural_cmp("REC-10", "REC-2"), Ordering::Greater);
        // 需求序：R-5.4（文档附录节）字典序在 SR-1 前，自然序一致
        assert_eq!(natural_cmp("R-5.4", "SR-1"), Ordering::Less);
    }

    #[test]
    fn anchored_order_uses_front_requirement_and_puts_orphans_last() {
        fn node(id: u32, kind: TraceNodeKind, reference: &str) -> TraceNode {
            let mut n = TraceNode::new(
                kind,
                "src".into(),
                reference.to_string(),
                None,
                reference.to_string(),
                None,
            );
            n.id = u32_to_uuid(id);
            n
        }
        // SR-1 rank0、SR-2 rank1（natural ref 序）。IT-B 同时 derives 两个需求 → 锚 SR-1；
        // IT-X 孤儿 → 排最后。
        let nodes = vec![
            node(1, TraceNodeKind::SoftwareRequirement, "SR-1"),
            node(2, TraceNodeKind::SoftwareRequirement, "SR-2"),
            node(3, TraceNodeKind::TestItem, "IT-1"),
            node(4, TraceNodeKind::TestItem, "IT-2"),
            node(5, TraceNodeKind::TestItem, "IT-B"),
            node(6, TraceNodeKind::TestItem, "IT-X"),
        ];
        let links = [
            e(3, 1, LinkType::DerivesFrom),
            e(4, 2, LinkType::DerivesFrom),
            e(5, 1, LinkType::DerivesFrom),
            e(5, 2, LinkType::DerivesFrom),
        ];
        let key: HashMap<Uuid, i32> = requirement_anchored_order(&nodes, &links)
            .into_iter()
            .collect();
        assert_eq!(key[&u32_to_uuid(1)], 0); // SR-1
        assert_eq!(key[&u32_to_uuid(2)], 1); // SR-2
        assert_eq!(key[&u32_to_uuid(3)], 0); // IT-1（锚 SR-1）
        assert_eq!(key[&u32_to_uuid(5)], 1); // IT-B（锚 SR-1 组内，排在 IT-1 后）
        assert_eq!(key[&u32_to_uuid(4)], 2); // IT-2（锚 SR-2 → rank1 组在 rank0 组后）
        assert_eq!(key[&u32_to_uuid(6)], 3); // IT-X（孤儿排最后）
    }

    #[test]
    fn uncovered_lists_software_requirements_without_incoming_trace() {
        fn node(id: u32, kind: TraceNodeKind) -> TraceNode {
            let mut n = TraceNode::new(
                kind,
                "src".into(),
                id.to_string(),
                None,
                format!("title {id}"),
                None,
            );
            n.id = u32_to_uuid(id);
            n
        }
        let nodes = vec![
            node(1, TraceNodeKind::SoftwareRequirement), // 无 incoming -> uncovered
            node(2, TraceNodeKind::SoftwareRequirement), // 被 3 derives -> covered
            node(3, TraceNodeKind::TestItem),
        ];
        let links = [e(3, 2, LinkType::DerivesFrom)];
        let uncovered = uncovered(&nodes, &links);
        assert_eq!(uncovered, vec![u32_to_uuid(1)]);
    }
}
