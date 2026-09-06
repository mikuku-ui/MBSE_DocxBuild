import { useCallback, useEffect, useMemo, useState } from 'react';
import { Tag, Typography } from 'antd';
import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  applyNodeChanges,
  type Edge,
  type Node,
  type NodeChange,
  type NodeProps,
} from '@xyflow/react';
import type { TemplateEdge, TemplateNode } from '../../app/types';
import { NODE_TYPE_LABEL } from '../../app/types';
import {
  GROUP_PAD,
  NODE_H,
  NODE_TYPE_COLOR,
  NODE_W,
  cmpNode,
  containsParentMap,
  nodeX,
  templateLayout,
} from './layout';

interface CardData {
  node: TemplateNode;
  selected: boolean;
  depth: number;
}

interface FrameData {
  color: string;
}

type RFNode = Node<Record<string, unknown>>;

function TemplateCardNode({ data }: NodeProps) {
  const d = data as unknown as CardData;
  const n = d.node;
  const color = NODE_TYPE_COLOR[n.node_type];
  const bg = d.selected ? `${color}1f` : '#ffffff';
  const handleStyle = { background: color, width: 10, height: 10 };
  const chainHandleStyle = { background: color, width: 6, height: 6, opacity: 0.55 };
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        background: bg,
        border: d.selected ? `2px solid ${color}` : `1px solid ${color}66`,
        borderLeft: `5px solid ${color}`,
        borderRadius: 8,
        padding: '6px 10px',
        cursor: 'pointer',
      }}
    >
      <Handle type="target" position={Position.Left} id="left" style={handleStyle} />
      <Handle type="source" position={Position.Right} id="right" style={handleStyle} />
      <Handle type="target" position={Position.Top} id="top" style={chainHandleStyle} />
      <Handle type="source" position={Position.Bottom} id="bottom" style={chainHandleStyle} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
        <Tag style={{ margin: 0, fontSize: 11 }} color={color}>
          {NODE_TYPE_LABEL[n.node_type]}
        </Tag>
      </div>
      <Typography.Text style={{ fontSize: 13, lineHeight: '18px' }} ellipsis={{ tooltip: n.title }}>
        {n.title || '（未命名）'}
      </Typography.Text>
    </div>
  );
}

function GroupFrameNode({ data }: NodeProps) {
  const d = data as unknown as FrameData;
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        background: `${d.color}0f`, // 约 6% 底
        border: `1px dashed ${d.color}55`,
        borderRadius: 10,
        pointerEvents: 'none',
        overflow: 'hidden',
      }}
    />
  );
}

const nodeTypes = { templateCard: TemplateCardNode, groupFrame: GroupFrameNode };

interface Props {
  nodes: TemplateNode[];
  edges: TemplateEdge[];
  selectedId: string | null;
  onSelectNode: (id: string | null) => void;
  /** 同组纵向拖拽松手 → 该组新顺序（id 列表），父组件落库并刷新 */
  onReorder: (ids: string[]) => void;
}

export default function TemplateCanvas({
  nodes,
  edges,
  selectedId,
  onSelectNode,
  onReorder,
}: Props) {
  const parentMap = useMemo(() => containsParentMap(edges), [edges]);
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  // 静态布局只用来给「初始坐标 + 每节点深度」；分组框/纵向链按实时节点位置现算。
  const { positions, depthOf } = useMemo(() => templateLayout(nodes, edges), [nodes, edges]);

  const [cards, setCards] = useState<RFNode[]>([]);

  // 数据变化即整体重算坐标（不做「保留旧 y」的增量），否则父节点被重排/结构变化时其子节点
  // 会滞留旧 y，出现「父与第一个子节点对不齐」。
  useEffect(() => {
    setCards(
      nodes.map((n) => {
        const depth = depthOf[n.id] ?? 0;
        return {
          id: n.id,
          type: 'templateCard',
          position: { x: nodeX(depth), y: positions[n.id]?.y ?? 0 },
          width: NODE_W,
          height: NODE_H,
          data: { node: n, selected: n.id === selectedId, depth },
          style: { width: NODE_W, height: NODE_H },
        } as RFNode;
      }),
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes, edges, selectedId]);

  // 分组背景框：每个父节点在其子列包住它的全部直接子节点，实时跟随子节点当前 y。
  const frames = useMemo<RFNode[]>(() => {
    const byParent: Record<string, RFNode[]> = {};
    for (const c of cards) {
      const p = parentMap[c.id];
      if (p !== undefined) (byParent[p] ??= []).push(c);
    }
    return Object.entries(byParent).map(([parentId, kids]) => {
      const depth = depthOf[parentId] ?? 0;
      const childX = nodeX(depth + 1);
      let top = Infinity;
      let bottom = -Infinity;
      for (const k of kids) {
        top = Math.min(top, k.position.y);
        bottom = Math.max(bottom, k.position.y + NODE_H);
      }
      const parent = byId.get(parentId);
      const color = parent ? NODE_TYPE_COLOR[parent.node_type] : '#8c8c8c';
      const width = NODE_W + 2 * GROUP_PAD;
      const height = bottom - top + 2 * GROUP_PAD;
      return {
        id: `group:${parentId}`,
        type: 'groupFrame',
        position: { x: childX - GROUP_PAD, y: top - GROUP_PAD },
        width,
        height,
        data: { color },
        selectable: false,
        draggable: false,
        focusable: false,
        style: { width, height },
      } as RFNode;
    });
  }, [cards, parentMap, depthOf, byId]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setCards((nds) => {
      const depthById = new Map(
        nds.map((n) => [n.id, (n.data as unknown as CardData)?.depth]),
      );
      const fixed = changes.map((ch) => {
        if (ch.type === 'position' && ch.position) {
          const depth = depthById.get(ch.id);
          if (depth !== undefined) {
            return { ...ch, position: { x: nodeX(depth), y: Math.max(0, ch.position.y) } };
          }
        }
        return ch;
      });
      return applyNodeChanges(fixed, nds);
    });
  }, []);

  // 拖拽松手：同组内按松手时 y 从上到下排成新序；顺序变了就乐观重排（本地重算布局立即
  // 回落）并落库，顺序没变则吸附回网格——与需求追踪「挪动后自己排序」的手感一致。
  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      const d = node.data as unknown as CardData | undefined;
      if (!d) return; // 分组背景框不可拖
      const id = node.id;
      const parent = parentMap[id];
      const siblings = cards
        .filter((c) => {
          const cd = c.data as unknown as CardData | undefined;
          return !!cd && parentMap[c.id] === parent;
        })
        .map((c) => c.id);
      const yOf = new Map(cards.map((c) => [c.id, c.position.y]));
      const byY = [...siblings].sort((a, b) => (yOf.get(a) ?? 0) - (yOf.get(b) ?? 0));
      const byDoc = [...siblings].sort((a, b) => {
        const na = byId.get(a);
        const nb = byId.get(b);
        return na && nb ? cmpNode(na, nb) : a.localeCompare(b);
      });
      const changed = siblings.length >= 2 && byY.some((cid, i) => cid !== byDoc[i]);
      if (changed) {
        // 乐观吸附：本地重编号 sort_key 并重算布局，让整棵树立马回到新序（不等后端往返）。
        const newKey = new Map(byY.map((cid, i) => [cid, i]));
        const optNodes = nodes.map((n) =>
          newKey.has(n.id) ? { ...n, sort_key: newKey.get(n.id)! } : n,
        );
        const optLayout = templateLayout(optNodes, edges);
        setCards((nds) =>
          nds.map((c) => {
            const pos = optLayout.positions[c.id];
            if (!pos) return c;
            const cd = c.data as unknown as CardData;
            const sk = newKey.has(c.id) ? newKey.get(c.id)! : cd.node.sort_key;
            return {
              ...c,
              position: { x: nodeX(cd.depth), y: pos.y },
              data: { ...cd, node: { ...cd.node, sort_key: sk } },
            };
          }),
        );
        onReorder(byY);
      } else {
        // 顺序没变（或没有可重排的兄弟）：吸附回网格。
        setCards((nds) =>
          nds.map((c) => {
            const pos = positions[c.id];
            if (!pos) return c;
            const cd = c.data as unknown as CardData;
            return { ...c, position: { x: nodeX(cd.depth), y: pos.y } };
          }),
        );
      }
    },
    [cards, parentMap, byId, nodes, edges, positions, onReorder],
  );

  const allNodes = useMemo(() => [...frames, ...cards], [frames, cards]);

  const liveIds = useMemo(() => new Set(nodes.map((n) => n.id)), [nodes]);
  // contains 边：父（右）→ 子（左）。
  const containsEdges: Edge[] = useMemo(
    () =>
      edges
        .filter((e) => liveIds.has(e.source_node_id) && liveIds.has(e.target_node_id))
        .map((e) => ({
          id: `e:${e.id}`,
          source: e.source_node_id,
          sourceHandle: 'right',
          target: e.target_node_id,
          targetHandle: 'left',
          type: 'smoothstep',
          style: { stroke: '#bfbfbf', strokeWidth: 1.5 },
        })),
    [edges, liveIds],
  );

  // 纵向链：同父（或同为一层）的相邻节点之间，从上往下串一条竖线（文档的「脊」）。
  const chainEdges: Edge[] = useMemo(() => {
    const byParent: Record<string, RFNode[]> = {};
    for (const c of cards) {
      const p = parentMap[c.id];
      (byParent[p ?? ''] ??= []).push(c);
    }
    const out: Edge[] = [];
    for (const group of Object.values(byParent)) {
      group.sort((a, b) => {
        const na = (a.data as unknown as CardData).node;
        const nb = (b.data as unknown as CardData).node;
        return cmpNode(na, nb);
      });
      for (let i = 0; i + 1 < group.length; i++) {
        out.push({
          id: `chain:${group[i].id}->${group[i + 1].id}`,
          source: group[i].id,
          sourceHandle: 'bottom',
          target: group[i + 1].id,
          targetHandle: 'top',
          type: 'straight',
          style: { stroke: '#c0c0c0', strokeWidth: 2 },
        });
      }
    }
    return out;
  }, [cards, parentMap]);

  const flowEdges = useMemo(
    () => [...containsEdges, ...chainEdges],
    [containsEdges, chainEdges],
  );

  return (
    <div style={{ height: '100%', width: '100%' }}>
      <ReactFlow
        nodes={allNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={(_, node) => {
          if (node.type === 'templateCard') onSelectNode(node.id);
        }}
        onPaneClick={() => onSelectNode(null)}
        nodesDraggable
        fitView
        fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
        deleteKeyCode={null}
        minZoom={0.2}
        maxZoom={1.5}
      >
        <Background gap={24} color="#e3e3e3" />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
