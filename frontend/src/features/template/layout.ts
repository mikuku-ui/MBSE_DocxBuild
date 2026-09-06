import type { TemplateEdge, TemplateNode, TemplateNodeType } from '../../app/types';

export interface XY {
  x: number;
  y: number;
}

export const NODE_TYPE_COLOR: Record<TemplateNodeType, string> = {
  frontmatter: '#722ed1',
  chapter: '#1677ff',
  section: '#13c2c2',
  paragraph: '#fa8c16',
  table: '#52c41a',
  figure: '#eb2f96',
  appendix: '#8c8c8c',
  trace_view: '#f5222d',
};

// 画布几何：深度 = 列（子章节向右平移进入下一个 level）。
export const NODE_W = 300;
export const NODE_H = 68;
export const H_GAP = 96; // 相邻深度列的列间距
export const ROW_GAP = 28; // 同组内相邻节点的纵向间距
export const GROUP_PAD = 16; // 分组背景框的内边距

/** 第 depth 列节点的 x（节点居中于本列）。列本身没有背景，分组框在下一列。 */
export function nodeX(depth: number): number {
  return depth * (NODE_W + H_GAP);
}

// 与后端 `DocumentService::ordered_nodes` 同序：sort_key → created_at → id。
export function cmpNode(a: TemplateNode, b: TemplateNode): number {
  return (
    a.sort_key - b.sort_key ||
    a.created_at.localeCompare(b.created_at) ||
    a.id.localeCompare(b.id)
  );
}

/** contains 边 → 子节点 → 父节点映射（source=父、target=子）。 */
export function containsParentMap(edges: TemplateEdge[]): Record<string, string> {
  const parent: Record<string, string> = {};
  for (const e of edges) {
    if (e.edge_type !== 'contains') continue;
    parent[e.target_node_id] = e.source_node_id;
  }
  return parent;
}

/** 某父节点（id）的直接子节点 id 列表（含无父的 root），按文档序排。 */
export function childrenMap(
  nodes: TemplateNode[],
  edges: TemplateEdge[],
): { children: Record<string, string[]>; roots: string[] } {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const children: Record<string, string[]> = {};
  const hasParent = new Set<string>();
  for (const e of edges) {
    if (e.edge_type !== 'contains') continue;
    (children[e.source_node_id] ??= []).push(e.target_node_id);
    hasParent.add(e.target_node_id);
  }
  const sortIds = (ids: string[]) =>
    ids.sort((a, b) => {
      const na = byId.get(a);
      const nb = byId.get(b);
      if (na && nb) return cmpNode(na, nb);
      return a.localeCompare(b);
    });
  for (const ids of Object.values(children)) sortIds(ids);
  const roots = sortIds(nodes.filter((n) => !hasParent.has(n.id)).map((n) => n.id));
  return { children, roots };
}

export interface TemplateLayout {
  positions: Record<string, XY>;
  depthOf: Record<string, number>;
}

/**
 * 层级布局（坐标不落库，渲染期按 contains 边现算）。
 *
 * 文档模板是一棵树：最左列 = 一级节点（Word 大纲一级），子节点向右平移进入下一个
 * level（深度 = 列）。同父节点的所有下一级子节点**纵向堆叠**成一个组（一个背景色框），
 * 父节点**顶部对齐**其第一个子节点（文档大纲：父标题在子节点组正上方）；不同一级节点下
 * 的二级节点都在同一列（同一个 x），只是分布在不同的组里。一级节点归「文档 root」这一组，
 * root 不画框。
 *
 * 环 / 悬空等未被任何 root 到达的节点兜底按 depth 0 追加在末尾（不报错）。
 */
export function templateLayout(
  nodes: TemplateNode[],
  edges: TemplateEdge[] = [],
): TemplateLayout {
  const { children, roots: initialRoots } = childrenMap(nodes, edges);

  // 深度：从 root 开始 DFS；环上节点以首次访问为准（防死循环）。
  const depthOf: Record<string, number> = {};
  const visited = new Set<string>();
  const assignDepth = (id: string, d: number) => {
    if (visited.has(id)) return;
    visited.add(id);
    depthOf[id] = d;
    for (const c of children[id] ?? []) assignDepth(c, d + 1);
  };
  const roots: string[] = [...initialRoots];
  for (const r of initialRoots) assignDepth(r, 0);
  // 兜底：未被任何 root 到达的节点（环）按 depth 0 追加。
  const leftover = nodes
    .filter((n) => !visited.has(n.id))
    .sort(cmpNode)
    .map((n) => n.id);
  for (const id of leftover) {
    roots.push(id);
    assignDepth(id, 0);
  }

  // 纵向堆叠、父节点顶部对齐其第一个子节点的整洁树摆放（文档大纲样式：父标题在子节点组上方）。
  const ROW_H = NODE_H + ROW_GAP;
  const positions: Record<string, XY> = {};
  const placed = new Set<string>();
  const place = (id: string, topY: number): number => {
    if (placed.has(id)) return topY + ROW_H; // 环上兜底：已摆放的不再递归
    placed.add(id);
    const x = nodeX(depthOf[id]);
    const kids = children[id] ?? [];
    if (kids.length === 0) {
      positions[id] = { x, y: topY };
      return topY + ROW_H;
    }
    let cursor = topY;
    for (const c of kids) {
      cursor = place(c, cursor);
    }
    positions[id] = { x, y: topY };
    return cursor;
  };
  let cursor = 0;
  for (const r of roots) cursor = place(r, cursor);

  return { positions, depthOf };
}
