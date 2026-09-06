import type { TraceNode, TraceNodeKind } from '../../app/types';
import { NODE_KIND_LABEL, STAGE_SUBLINE } from '../../app/types';

export interface XY {
  x: number;
  y: number;
}

/** 整列重排提交载荷：拖拽松手 / 左栏上移下移时，把某阶段整列重新编号 0..n-1。 */
export interface SortOrderItem {
  id: string;
  sort_key: number;
}

// 五阶段泳道：数组下标小 = 越靠左 = 越低层；最右（顶层）= 需求。
// 左侧阶段的产物用于覆盖/回答其右侧阶段。
export const STAGE_ORDER: TraceNodeKind[] = [
  'test_report', // 0 最左
  'test_record', // 1
  'test_case', // 2
  'test_item', // 3
  'software_requirement', // 4 最右（顶层）
];

export const KIND_COLOR: Record<TraceNodeKind, string> = {
  test_report: '#eb2f96',
  test_record: '#faad14',
  test_case: '#52c41a',
  test_item: '#13c2c2',
  software_requirement: '#1890ff',
};

export const NODE_W = 300;
export const LANE_W = 440; // 一列泳道宽度（含节点两侧留白）
export const ROW_H = 116; // 同列节点纵向间距
export const HEADER_H = 92; // 泳道顶部标题区，节点纵向排版从其下方开始
const TOP_PAD = 18;

export function stageIndex(kind: TraceNodeKind): number {
  return STAGE_ORDER.indexOf(kind);
}

/** 某阶段泳道的左边界 x */
export function laneLeft(kind: TraceNodeKind): number {
  return stageIndex(kind) * LANE_W;
}

/** 节点 x：固定居中于所属泳道（节点不可横向越列，拖拽被限制在纵向） */
export function nodeX(kind: TraceNodeKind): number {
  return laneLeft(kind) + (LANE_W - NODE_W) / 2;
}

export interface StageMeta {
  kind: TraceNodeKind;
  index: number;
  label: string;
  subline: string;
  color: string;
  fill: string;
  border: string;
}

export const STAGE_META: Record<TraceNodeKind, StageMeta> = Object.fromEntries(
  STAGE_ORDER.map((kind, index) => [
    kind,
    {
      kind,
      index,
      label: NODE_KIND_LABEL[kind],
      subline: STAGE_SUBLINE[kind],
      color: KIND_COLOR[kind],
      fill: `${KIND_COLOR[kind]}1a`, // 10% 泳道底
      border: `${KIND_COLOR[kind]}40`,
    },
  ]),
) as Record<TraceNodeKind, StageMeta>;

/** 数字感知比较：TC-2 < TC-10（避免字典序把 TC-10 排在 TC-2 前）。 */
export function naturalCompare(a: string, b: string): number {
  type Tok = { kind: 'num'; value: number } | { kind: 'str'; value: string };
  const tokenize = (s: string): Tok[] => {
    const out: Tok[] = [];
    const re = /\d+|\D+/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(s))) {
      const seg = m[0];
      if (/^\d+$/.test(seg)) out.push({ kind: 'num', value: Number(seg) });
      else out.push({ kind: 'str', value: seg });
    }
    return out;
  };
  const ta = tokenize(a);
  const tb = tokenize(b);
  for (let i = 0; i < Math.min(ta.length, tb.length); i++) {
    const x = ta[i];
    const y = tb[i];
    if (x.kind === 'num' && y.kind === 'num') {
      if (x.value !== y.value) return x.value - y.value;
    } else if (x.kind !== y.kind) {
      return x.kind === 'num' ? -1 : 1;
    } else if (x.value !== y.value) {
      return x.value < y.value ? -1 : 1;
    }
  }
  return ta.length - tb.length;
}

/** 列内排序键：`sort_key`（用户自定，落库）主序 → 自然序 ref 兜底（如全为 0 时）。 */
export function stageSortKey(a: TraceNode, b: TraceNode): number {
  return (
    a.sort_key - b.sort_key ||
    naturalCompare(a.external_ref, b.external_ref) ||
    a.external_source.localeCompare(b.external_source) ||
    a.id.localeCompare(b.id)
  );
}

/** 自动布局：每列按 sort_key（用户顺序，落库）排布为纵向网格。坐标本身不落库，
 *  顺序落库于 `trace_nodes.sort_key`，这里只把序号翻译成像素 y。 */
export function tracePositions(nodes: TraceNode[]): Record<string, XY> {
  const groups: Partial<Record<TraceNodeKind, TraceNode[]>> = {};
  for (const n of nodes) (groups[n.kind] ??= []).push(n);
  const positions: Record<string, XY> = {};
  for (const kind of STAGE_ORDER) {
    const arr = [...(groups[kind] ?? [])].sort(stageSortKey);
    arr.forEach((n, i) => {
      positions[n.id] = { x: nodeX(kind), y: HEADER_H + TOP_PAD + i * ROW_H };
    });
  }
  return positions;
}
