import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from 'react';
import {
  App,
  Button,
  Popconfirm,
  Space,
  Tag,
  Typography,
} from 'antd';
import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  applyNodeChanges,
  type Connection,
  type Edge,
  type Node,
  type NodeChange,
  type NodeProps,
} from '@xyflow/react';
import type { TraceLink, TraceNode, TraceNodeKind } from '../../app/types';
import { LINK_TYPE_LABEL } from '../../app/types';
import type { SortOrderItem } from './layout';
import ContextMenu, { clampMenuPos, type ContextMenuItem } from '../../components/ContextMenu';
import {
  HEADER_H,
  KIND_COLOR,
  LANE_W,
  NODE_W,
  ROW_H,
  STAGE_META,
  STAGE_ORDER,
  laneLeft,
  nodeX,
  stageIndex,
  stageSortKey,
  tracePositions,
} from './layout';

interface TraceCardData {
  trace: TraceNode;
  marked: boolean;
  selected: boolean;
}

type RFNode = Node<Record<string, unknown>>;

const CARD_H = 84;
// 节点可拖动的纵向范围：上界 = 列头下方（不能盖住/越过列头），下界不设限
// （泳道背景会实时跟随节点往下延伸）。
const NODE_MIN_Y = HEADER_H + 6;
// 网格排版顶（与 layout.ts 的 TOP_PAD 对齐）：第 i 行中心 y = GRID_TOP + i*ROW_H。
const GRID_TOP = HEADER_H + 18;

function traceKindOf(n: RFNode): TraceNodeKind | undefined {
  return (n.data as unknown as TraceCardData)?.trace?.kind;
}

function TraceCardNode({ data }: NodeProps) {
  const d = data as unknown as TraceCardData;
  const t = d.trace;
  const color = KIND_COLOR[t.kind];
  const bg = d.selected ? `${color}1f` : '#ffffff';
  return (
    <div
      style={{
        width: '100%',
        minHeight: CARD_H,
        boxSizing: 'border-box',
        background: bg,
        // 高亮（向上追溯/向下影响面的链）优先于选中：整体黑描边 + 黑色小色带 + 淡黑外环，
        // 比之前的淡橙 box-shadow 更醒目。
        border: d.marked
          ? '2px solid #000'
          : d.selected
            ? `2px solid ${color}`
            : `1px solid ${color}66`,
        borderLeft: d.marked ? '5px solid #000' : `5px solid ${color}`,
        borderRadius: 8,
        boxShadow: d.marked ? '0 0 0 2px rgba(0,0,0,0.18)' : undefined,
        padding: '6px 10px',
        cursor: 'pointer',
      }}
    >
      <Handle
        type="target"
        position={Position.Left}
        style={{ background: color, width: 10, height: 10 }}
        title="连入点：接收来自左侧（下级阶段）的追溯边"
      />
      <Handle
        type="source"
        position={Position.Right}
        style={{ background: color, width: 10, height: 10 }}
        title="连出点：从这里拖向右侧（上级阶段）建追溯边"
      />
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
        <Tag style={{ margin: 0, fontSize: 11 }} color={color}>
          {t.external_ref}
        </Tag>
        <span style={{ fontSize: 10, color: '#bfbfbf' }}>{t.external_source}</span>
      </div>
      <Typography.Text
        style={{ fontSize: 13, lineHeight: '18px' }}
        ellipsis={{ tooltip: t.title }}
      >
        {t.title || '（未命名）'}
      </Typography.Text>
      <div style={{ marginTop: 2, fontSize: 11, color: '#8c8c8c', height: 16, overflow: 'hidden' }}>
        {t.module ? `模块：${t.module}` : ' '}
      </div>
    </div>
  );
}

function StageBandNode({ data }: NodeProps) {
  const d = data as unknown as { meta: (typeof STAGE_META)[TraceNodeKind]; height: number };
  const { meta } = d;
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        background: meta.fill,
        borderRight: `1px dashed ${meta.border}`,
        borderLeft: `1px solid ${meta.border}`,
        pointerEvents: 'none',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          height: HEADER_H,
          boxSizing: 'border-box',
          padding: '10px 14px',
          background: `linear-gradient(180deg, ${meta.fill}, transparent)`,
          borderBottom: `2px solid ${meta.color}`,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span
            style={{
              display: 'inline-block',
              width: 12,
              height: 12,
              borderRadius: 3,
              background: meta.color,
            }}
          />
          <Typography.Text strong style={{ fontSize: 16, color: meta.color }}>
            {meta.label}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 11 }}>
            {meta.subline}
          </Typography.Text>
        </div>
        <Typography.Text type="secondary" style={{ fontSize: 10 }}>
          本列内节点可纵向移动，不会越出该阶段列
        </Typography.Text>
      </div>
    </div>
  );
}

const nodeTypes = { traceCard: TraceCardNode, stageBand: StageBandNode };

interface Props {
  nodes: TraceNode[];
  links: TraceLink[];
  selectedId: string | null;
  highlight: Set<string>;
  onSelectNode: (id: string | null) => void;
  /** 双击 / 右键「编辑」（仅需求节点会触发） */
  onEditNode: (id: string) => void;
  /** 右键「删除」（节点级确认在父组件） */
  onDeleteNode: (node: TraceNode) => void;
  /** 右键「向上追溯 / 向下影响面」 */
  onReachNode: (node: TraceNode, dir: 'up' | 'down') => void;
  /** 正在执行哪个方向的溯源（独立 loading） */
  reachBusy: 'up' | 'down' | null;
  /** 建追溯边（两端点已归一化为「低层→高层」） */
  onConnectEdge: (sourceId: string, targetId: string) => void;
  onDeleteLink: (linkId: string) => void;
  /** 拖拽松手 → 该阶段整列新顺序（重编号），父组件落库并刷新 */
  onReorder: (orders: SortOrderItem[]) => void;
}

export default function TraceCanvas({
  nodes,
  links,
  selectedId,
  highlight,
  onSelectNode,
  onEditNode,
  onDeleteNode,
  onReachNode,
  reachBusy,
  onConnectEdge,
  onDeleteLink,
  onReorder,
}: Props) {
  const { message } = App.useApp();
  const wrapRef = useRef<HTMLDivElement>(null);
  const [contentNodes, setContentNodes] = useState<RFNode[]>([]);
  const [selEdge, setSelEdge] = useState<TraceLink | null>(null);
  // 画布节点右键操作菜单
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; node: TraceNode } | null>(
    null,
  );

  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);

  // 服务端数据变化时重排。节点保留先前的 y 仅当「没被重新排序」——重排/整理后
  // （sort_key 变了）应回落到 sort_key 决定的网格位置，而不是赖在旧 y 上。
  useEffect(() => {
    setContentNodes((prev) => {
      const prevById = new Map(prev.map((n) => [n.id, n]));
      const layout = tracePositions(nodes);
      return nodes.map((n) => {
        const prior = prevById.get(n.id);
        const priorData = prior?.data as unknown as TraceCardData | undefined;
        const keepY =
          prior &&
          priorData?.trace.kind === n.kind &&
          priorData.trace.sort_key === n.sort_key;
        const y = keepY && prior ? prior.position.y : (layout[n.id]?.y ?? 0);
        return {
          id: n.id,
          type: 'traceCard',
          position: { x: nodeX(n.kind), y },
          width: NODE_W,
          height: CARD_H,
          data: {
            trace: n,
            marked: highlight.has(n.id),
            selected: n.id === selectedId,
          },
          style: { width: NODE_W, height: CARD_H },
        } as RFNode;
      });
    });
  }, [nodes, selectedId, highlight]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setContentNodes((nds) => {
      const kindById = new Map(
        nds.map((n) => [n.id, (n.data as unknown as TraceCardData)?.trace?.kind]),
      );
      const fixed = changes.map((ch) => {
        if (ch.type === 'position' && ch.position) {
          const kind = kindById.get(ch.id) as TraceNodeKind | undefined;
          if (kind) {
            // 横向锁定在本阶段列；纵向夹在列头之下、背景跟随延伸
            const y = Math.max(NODE_MIN_Y, ch.position.y);
            return { ...ch, position: { x: nodeX(kind), y } };
          }
        }
        return ch;
      });
      return applyNodeChanges(fixed, nds);
    });
  }, []);

  // 拖拽松手：本列按松手时的 y 从上到下排成新序，吸附到网格；顺序与之前不同则
  // 提交整列重编号（onReorder → 父组件 PUT /trace/nodes/order 落库）。
  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      const kind = traceKindOf(node);
      if (!kind) return;
      const cols = contentNodes.filter((c) => traceKindOf(c) === kind);
      const byY = [...cols]
        .sort((a, b) => a.position.y - b.position.y)
        .map((c) => c.id);
      const prev = [...cols]
        .sort((a, b) =>
          stageSortKey(
            (a.data as unknown as TraceCardData).trace,
            (b.data as unknown as TraceCardData).trace,
          ),
        )
        .map((c) => c.id);
      const changed = byY.some((id, i) => id !== prev[i]);

      // 乐观吸附：先把这一列按新 y 序落到网格行（即便顺序没变也把拖歪的节点对齐）
      setContentNodes((nds) => {
        const row = new Map(byY.map((id, i) => [id, i]));
        return nds.map((c) => {
          const r = row.get(c.id);
          if (r === undefined) return c;
          return { ...c, position: { x: nodeX(kind), y: GRID_TOP + r * ROW_H } };
        });
      });
      if (changed) {
        onReorder(byY.map((id, i) => ({ id, sort_key: i })));
      }
    },
    [contentNodes, onReorder],
  );

  // 右键节点 → 操作菜单（编辑仅需求节点；追溯/影响面/删除全阶段可用）
  const onNodeContextMenu = useCallback((e: ReactMouseEvent, node: Node) => {
    e.preventDefault();
    const t = (node.data as unknown as TraceCardData)?.trace;
    if (!t) return;
    const rect = wrapRef.current?.getBoundingClientRect();
    if (!rect) return;
    const menuW = 168;
    const menuH = t.kind === 'software_requirement' ? 212 : 150;
    const pos = clampMenuPos(e.clientX, e.clientY, rect, menuW, menuH);
    setCtxMenu({ x: pos.x, y: pos.y, node: t });
  }, []);

  const closeCtxMenu = useCallback(() => setCtxMenu(null), []);

  // 泳道背景：跟随该列节点的实时纵向范围（节点拖多低，背景就延伸到哪）。
  // 关键点：泳道是显式给了 width/height 的节点（React Flow 直接用，不再 DOM 测量），
  // 且不设负 zIndex——所以拖动过程中每次重算高度也不会闪烁/消失。
  const bands = useMemo(() => {
    const bottom: Partial<Record<TraceNodeKind, number>> = {};
    for (const n of contentNodes) {
      const data = n.data as unknown as TraceCardData;
      const k = data?.trace?.kind;
      if (!k) continue;
      bottom[k] = Math.max(bottom[k] ?? 0, n.position.y + CARD_H);
    }
    return STAGE_ORDER.map((kind) => {
      const height = Math.max(HEADER_H + 280, (bottom[kind] ?? 0) + 56);
      return {
        id: `stage:${kind}`,
        type: 'stageBand',
        position: { x: laneLeft(kind), y: 0 },
        width: LANE_W,
        height,
        data: { meta: STAGE_META[kind] },
        style: { width: LANE_W, height },
        selectable: false,
        draggable: false,
        focusable: false,
      } as RFNode;
    });
  }, [contentNodes]);

  const flowEdges: Edge[] = useMemo(() => {
    const live = new Set(contentNodes.map((n) => n.id));
    return links
      .filter((l) => live.has(l.source_node_id) && live.has(l.target_node_id))
      .map((l) => {
        const active = selEdge?.id === l.id;
        const hl =
          highlight.has(l.source_node_id) && highlight.has(l.target_node_id);
        return {
          id: `l:${l.id}`,
          source: l.source_node_id,
          target: l.target_node_id,
          label: LINK_TYPE_LABEL[l.link_type],
          labelStyle: { fontSize: 10, fill: active ? '#fa541c' : '#8c8c8c' },
          labelBgStyle: { fill: '#fff', fillOpacity: 0.92 },
          style: {
            // 高亮链上的追溯边同步黑色加粗，与节点黑边框呼应
            stroke: active ? '#fa541c' : hl ? '#000' : '#b5b5b5',
            strokeWidth: active ? 2.5 : hl ? 2.5 : 1.5,
          },
        };
      });
  }, [links, selEdge, highlight, contentNodes]);

  const allNodes = useMemo(() => [...bands, ...contentNodes], [bands, contentNodes]);

  function handleConnect(conn: Connection) {
    const s = conn.source ? byId.get(conn.source) : undefined;
    const t = conn.target ? byId.get(conn.target) : undefined;
    if (!s || !t || s.id === t.id) return;
    if (s.kind === t.kind) {
      message.warning('同一阶段列内的节点之间不需要连线（上下节点无连接需求）');
      return;
    }
    // 归一化：无论从哪端拖起，都存成 低层(左)→高层(右)
    const lo = stageIndex(s.kind) < stageIndex(t.kind) ? s : t;
    const hi = stageIndex(s.kind) < stageIndex(t.kind) ? t : s;
    onConnectEdge(lo.id, hi.id);
  }

  return (
    <div ref={wrapRef} style={{ position: 'relative', height: '100%', width: '100%' }}>
      <ReactFlow
        nodes={allNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onNodeDragStop={onNodeDragStop}
        onNodeContextMenu={onNodeContextMenu}
        onNodeClick={(_, node) => {
          onSelectNode(node.id);
          setSelEdge(null);
          closeCtxMenu();
        }}
        onNodeDoubleClick={(_, node) => onEditNode(node.id)}
        onPaneClick={() => {
          onSelectNode(null);
          setSelEdge(null);
          closeCtxMenu();
        }}
        onConnect={handleConnect}
        onEdgeClick={(_, edge) =>
          setSelEdge(links.find((l) => `l:${l.id}` === edge.id) ?? null)
        }
        fitView
        fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
        deleteKeyCode={null}
        minZoom={0.2}
        maxZoom={1.5}
      >
        <Background gap={24} color="#e3e3e3" />
        <Controls showInteractive={false} />
      </ReactFlow>

      {selEdge && (
        <div
          style={{
            position: 'absolute',
            top: 12,
            left: '50%',
            transform: 'translateX(-50%)',
            zIndex: 20,
            display: 'flex',
            gap: 8,
            alignItems: 'center',
            background: '#fff',
            padding: '4px 10px',
            borderRadius: 8,
            boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
          }}
        >
          <Typography.Text style={{ fontSize: 12 }}>
            {byId.get(selEdge.source_node_id)?.external_ref ?? '?'}
            <Typography.Text type="secondary">
              {' '}—{LINK_TYPE_LABEL[selEdge.link_type]}→{' '}
            </Typography.Text>
            {byId.get(selEdge.target_node_id)?.external_ref ?? '?'}
          </Typography.Text>
          <Popconfirm
            title="删除这条追溯边？"
            okText="删除"
            cancelText="取消"
            okButtonProps={{ danger: true }}
            onConfirm={() => {
              onDeleteLink(selEdge.id);
              setSelEdge(null);
            }}
          >
            <Button size="small" danger>
              删除边
            </Button>
          </Popconfirm>
          <Button size="small" onClick={() => setSelEdge(null)}>
            取消
          </Button>
        </div>
      )}

      {/* 节点右键操作菜单 */}
      {ctxMenu && (
        <ContextMenu
          title={`${ctxMenu.node.external_ref} · ${ctxMenu.node.title}`}
          x={ctxMenu.x}
          y={ctxMenu.y}
          onClose={closeCtxMenu}
          items={
            [
              {
                key: 'up',
                label: '向上追溯',
                disabled: reachBusy !== null,
                loading: reachBusy === 'up',
                onClick: () => onReachNode(ctxMenu.node, 'up'),
              },
              {
                key: 'down',
                label: '向下影响面',
                disabled: reachBusy !== null,
                loading: reachBusy === 'down',
                onClick: () => onReachNode(ctxMenu.node, 'down'),
              },
              ...(ctxMenu.node.kind === 'software_requirement'
                ? [
                    {
                      key: 'edit',
                      label: '编辑',
                      onClick: () => onEditNode(ctxMenu.node.id),
                    },
                  ]
                : []),
              {
                key: 'delete',
                label: '删除',
                danger: true,
                onClick: () => onDeleteNode(ctxMenu.node),
              },
            ] as ContextMenuItem[]
          }
        />
      )}
    </div>
  );
}
