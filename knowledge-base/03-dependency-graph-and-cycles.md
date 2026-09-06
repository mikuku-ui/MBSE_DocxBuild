# 03 · 依赖图：关系模型、环检测（Dependency Graph & Cycle Guard）

> 源码锚点：`backend/src/domain/relationship/entity.rs` · `backend/src/application/workflow/service.rs`

## 1. 关系类型与语义分工

两个任务之间可以存在一条有向关系边：`source ──▶ target`。

```
Relationship { source: TaskId, target: TaskId, type: RelationshipType }

RelationshipType
 ├─ Dependency    = “source 必须先于 target”——**有执行语义**（进排序/健康/布局）
 └─ Reference     = “source 关联到 target”——纯引用，**无执行语义**（全部忽略）
```

### R7 · 只有 dependency 有业务语义
| 规则/机制 | Dependency | Reference |
|---|---|---|
| 成环校验 | ✅ 禁止成环 | ❌ 不做（环引用可存在） |
| 拓扑执行顺序（06） | ✅ 参与 | 忽略 |
| dependency_health（05） | ✅ 参与 | 忽略 |
| 画布自动分组/分层（07） | ✅ 参与（弱连通+分层） | 忽略 |

> 一句话：**reference 是给产品画“相关连”用的，业务图只认 dependency。**

## 2. 关系边不变式（R6）

创建关系时按顺序校验，任一条失败 → `Conflict`（或 Validation）并拒建：

```text
1. source ≠ target                        （自环 → Validation/Conflict）
2. source 存在 && target 存在              （否则 NotFound）
3. (source, target, type) 尚未存在          （重复 → Conflict）
4. type == Dependency 时：不得成环           （将成环 → Conflict，见下）
```

> 注意去重键含 type：同一对任务可以**同时**有一条 dependency 和一条 reference，互不冲突。

## 3. 环检测算法（R8）

新增边记为 `source → target`（`source` 是被依赖方）。闭合成环 ⇔ **在“不含新边”的现有图里，已经存在一条从 `target` 到 `source` 的 dependency 路径**（否则 `source →…→ target` 加上新边 `target ⇝ source` 会闭合）。

> 等价判据（实现所用）：`reachable(existing_graph, from = target, to = source)`。

```text
会不会成环？  ⟺  target ⇝ source（只沿 dependency 可达）？

现有： A ──▶ B ──▶ C
  加 A → C : 从 C 出发能到 A 吗？ 否                    → ✅ 允许（只是补了一条捷径）
  加 B → A : 从 A 出发能到 B 吗？ A→B 可达             → ❌ 拒绝（成环 A→B→A）
  加 C → A : 从 A 出发能到 C 吗？ A→B→C 可达          → ❌ 拒绝（成环 A→B→C→A）
  加 C → B : 从 B 出发能到 C 吗？ B→C 可达             → ❌ 拒绝（成环 B→C→B）
```

### 最小可移植实现
```rust
// edges: 现有 dependency 边；新边 = source → target
// 成环 ⇔ 从 target 沿现有边能到达 source
fn would_create_cycle(source, target, edges) -> bool {
    let successors = adjacency(edges);         // node -> 出边 targets
    let mut stack = vec![target];               // 从 target 出发
    let mut seen = HashSet::new();
    while let Some(n) = stack.pop() {
        if n == source { return true; }
        if !seen.insert(n) { continue; }
        for succ in successors[n] { stack.push(succ); }
    }
    false
}
```
> 只把 **dependency** 边喂进去；reference 永远不参与。现有图始终是 DAG（每条边都先验过环），BFS/DFS 不会死循环。

## 4. 删除关系 / 任务后的自愈

- 删一条边：无副作用（图只会更“松散”）。
- 删一个任务：连带的**依赖边整条删除**（不是置为悬挂）；
  仍可能残留“端点已不存在的边”时（如外部导入脏数据），下游算法**忽略悬挂边**而非崩溃：
  - 拓扑排序（06）：不认识的任务直接跳过该边；
  - 健康度（05）：递归自然返回“不阻塞”（没有该任务 = 视为无前置影响）；
  - 自动布局（07）：`parent.has` 两端才并集。

## 5. 语义注释

图方向约定（务必在下一个项目沿用，否则会反）：
- **dependency：`source` 是被依赖方（先做完的），`target` 是依赖方（等 source 的）**。
- 因此在 UI 里“前置任务画在左边/上边，后置任务靠右/靠下”，source 总是在 target 之前执行。
