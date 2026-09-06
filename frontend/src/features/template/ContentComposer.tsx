import { useEffect, useMemo, useState } from 'react';
import { App, Button, Divider, Empty, Input, Select, Space, Tag, Typography } from 'antd';
import { api } from '../../app/api';
import { errMsg } from '../../app/format';
import { useAppDispatch } from '../../app/hooks';
import type {
  Scheme,
  SchemeFull,
  TemplateEdge,
  TemplateNode,
} from '../../app/types';
import {
  BLOCK_KIND_LABEL,
  DOC_KIND_OPTIONS,
  NODE_TYPE_LABEL,
  TRACE_VIEW_LABEL,
} from '../../app/types';
import { NODE_TYPE_COLOR, childrenMap, cmpNode } from './layout';
import { updateNode } from './templateSlice';

// ---------------------------------------------------------------------------
// 五原语编辑器状态（与后端 ContentSpec 同构；kind='none' = 无内容，存 {}）
// ---------------------------------------------------------------------------

type SpecKind = 'none' | 'text' | 'field' | 'table' | 'trace' | 'slot';

interface SpecState {
  kind: SpecKind;
  text: string;
  field: string;
  doc_kind?: string;
  source?: string;
  party_kind?: string;
  category?: string;
  view?: string;
  placeholder?: string;
  role?: string;
}

const KIND_LABEL: Record<SpecKind, string> = {
  none: BLOCK_KIND_LABEL.empty,
  text: BLOCK_KIND_LABEL.text,
  field: BLOCK_KIND_LABEL.field,
  table: BLOCK_KIND_LABEL.table,
  trace: BLOCK_KIND_LABEL.trace,
  slot: BLOCK_KIND_LABEL.slot,
};
const KIND_OPTIONS = (['none', 'text', 'field', 'table', 'trace', 'slot'] as SpecKind[]).map(
  (k) => ({ value: k, label: KIND_LABEL[k] }),
);

// 表来源 = 各体系自定义表的并集（组件内按 GET /api/schemes + 各完整定义动态取）；
// party_kind / category 筛选列是否可用，取决于所选表在体系里有没有同名列。
interface SchemeTableColMeta {
  label: string;
  hasPartyKind: boolean;
  hasCategory: boolean;
}

const TRACE_VIEW_OPTIONS = (['matrix', 'test_items'] as const).map((v) => ({
  value: v,
  label: TRACE_VIEW_LABEL[v],
}));

/** 已有 content_spec（jsonb 宽松对象）→ 编辑器状态；未知/缺字段按 none。 */
function parseSpec(raw: Record<string, unknown> | undefined | null): SpecState {
  const kind = typeof raw?.kind === 'string' ? raw.kind : '';
  const s = (k: string): string => (raw && typeof raw[k] === 'string' ? (raw[k] as string) : '');
  const opt = (k: string): string | undefined => {
    const v = s(k);
    return v ? v : undefined;
  };
  switch (kind) {
    case 'text':
      return { kind: 'text', text: s('text'), field: '' };
    case 'field':
      return { kind: 'field', text: '', field: s('field'), doc_kind: opt('doc_kind') };
    case 'table':
      return {
        kind: 'table',
        text: '',
        field: '',
        source: s('source') || undefined,
        party_kind: opt('party_kind'),
        category: opt('category'),
      };
    case 'trace':
      return { kind: 'trace', text: '', field: '', view: s('view') || 'matrix' };
    case 'slot':
      return {
        kind: 'slot',
        text: '',
        field: '',
        placeholder: s('placeholder'),
        role: opt('role'),
      };
    default:
      return { kind: 'none', text: '', field: '' };
  }
}

/** 编辑器状态 → 待存 content_spec（none → {} = 清空）。 */
function toPayload(
  st: SpecState,
  col?: { hasPartyKind: boolean; hasCategory: boolean },
): Record<string, unknown> {
  switch (st.kind) {
    case 'text':
      return { kind: 'text', text: st.text };
    case 'field': {
      const out: Record<string, unknown> = { kind: 'field', field: st.field };
      if (st.doc_kind) out.doc_kind = st.doc_kind;
      return out;
    }
    case 'table': {
      const out: Record<string, unknown> = { kind: 'table', source: st.source ?? '' };
      // 只落「该表在体系里确有对应列」的筛选；否则装配会误伤（无该列的行全被滤掉）。
      if (col?.hasPartyKind && st.party_kind) out.party_kind = st.party_kind;
      if (col?.hasCategory && st.category) out.category = st.category;
      return out;
    }
    case 'trace':
      return { kind: 'trace', view: st.view ?? 'matrix' };
    case 'slot': {
      const out: Record<string, unknown> = { kind: 'slot', placeholder: st.placeholder ?? '' };
      if (st.role) out.role = st.role;
      return out;
    }
    default:
      return {};
  }
}

// ---------------------------------------------------------------------------
// 文档序列表（对齐后端 ordered_nodes：sort_key → created_at → id）
// ---------------------------------------------------------------------------

function docOrder(
  nodes: TemplateNode[],
  edges: TemplateEdge[],
): { ids: string[]; depthOf: Record<string, number> } {
  const { children, roots } = childrenMap(nodes, edges);
  const ids: string[] = [];
  const depthOf: Record<string, number> = {};
  const seen = new Set<string>();
  const walk = (id: string, depth: number) => {
    if (seen.has(id)) return;
    seen.add(id);
    ids.push(id);
    depthOf[id] = depth;
    for (const c of children[id] ?? []) walk(c, depth + 1);
  };
  for (const r of roots) walk(r, 0);
  // 兜底：环上未被任何 root 到达的节点，按文档序补在末尾
  const leftover = nodes
    .filter((n) => !seen.has(n.id))
    .sort(cmpNode)
    .map((n) => n.id);
  for (const id of leftover) {
    ids.push(id);
    depthOf[id] = 0;
  }
  return { ids, depthOf };
}

// ---------------------------------------------------------------------------
// 组件：左 = 文档序节点列表；右 = 选中节点五原语编辑器
// ---------------------------------------------------------------------------

interface ContentComposerProps {
  templateId: string;
  nodes: TemplateNode[];
  edges: TemplateEdge[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
}

export default function ContentComposer({
  templateId,
  nodes,
  edges,
  selectedId,
  onSelect,
}: ContentComposerProps) {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();

  const nodeById = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  const { ids, depthOf } = useMemo(() => docOrder(nodes, edges), [nodes, edges]);
  const selNode = selectedId ? nodeById.get(selectedId) : undefined;

  const [spec, setSpec] = useState<SpecState>({ kind: 'none', text: '', field: '' });

  // 表来源：各体系自定义表并集（跨全部体系合并去重）；筛选列标记视所选表而定。
  const [colMeta, setColMeta] = useState<Map<string, SchemeTableColMeta>>(new Map());
  const [tableOptions, setTableOptions] = useState<Array<{ value: string; label: string }>>([]);

  // 切换选中节点 → 载入其 content_spec
  useEffect(() => {
    setSpec(parseSpec(selNode?.content_spec));
  }, [selNode]);

  // 拉体系并集（表来源下拉 + 筛选列能力）
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const metas = await api.get<Scheme[]>('/schemes');
        const fulls = await Promise.all(
          metas.map((m) => api.get<SchemeFull>(`/schemes/${m.id}`)),
        );
        if (cancelled) return;
        const union = new Map<string, SchemeTableColMeta>();
        for (const f of fulls) {
          for (const td of f.tables) {
            const key = td.table.table_key;
            const prev = union.get(key);
            const rec: SchemeTableColMeta = {
              label: prev?.label || td.table.label || key,
              hasPartyKind: prev?.hasPartyKind || false,
              hasCategory: prev?.hasCategory || false,
            };
            for (const c of td.columns) {
              if (c.column_key === 'party_kind') rec.hasPartyKind = true;
              if (c.column_key === 'category') rec.hasCategory = true;
            }
            union.set(key, rec);
          }
        }
        setTableOptions(
          [...union.entries()]
            .map(([value, m]) => ({ value, label: `${m.label}（${value}）` }))
            .sort((a, b) => a.label.localeCompare(b.label, 'zh')),
        );
        setColMeta(union);
      } catch {
        /* 体系拉取失败：表来源先空着，不阻断其它五原语 */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function save() {
    if (!templateId || !selNode) return;
    try {
      const updated = await dispatch(
        updateNode({
          template_id: templateId,
          node_id: selNode.id,
          content_spec: toPayload(
            spec,
            spec.source ? colMeta.get(spec.source) : undefined,
          ),
        }),
      ).unwrap();
      setSpec(parseSpec(updated.content_spec));
      message.success('内容声明已保存（切到「3. 装配预览」看效果）');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  const patch = (p: Partial<SpecState>) => setSpec((s) => ({ ...s, ...p }));

  // 所选表在体系里的列能力（决定可否按 party_kind / category 列筛选）
  const tcol = spec.source ? colMeta.get(spec.source) : undefined;

  return (
    <div className="flex h-[calc(100vh-266px)] gap-2">
      {/* 左：文档序节点列表 */}
      <div className="w-[320px] flex-none overflow-auto rounded border border-gray-200 bg-white p-2">
        <Space direction="vertical" className="w-full!" size={6}>
          <Typography.Text strong className="px-1!">
            文档组成部分（按文档序，自上而下）
          </Typography.Text>
          <Typography.Text type="secondary" className="px-1! text-xs">
            点选节点 → 在右侧声明它的内容从哪来（五原语）。
          </Typography.Text>
          <Divider className="mb-1! mt-1!" />
          {ids.length === 0 && <Empty description="此模板还没有节点，先到「1. 文档结构」搭骨架。" />}
          {ids.map((id) => {
            const n = nodeById.get(id);
            if (!n) return null;
            const active = id === selectedId;
            return (
              <button
                key={id}
                type="button"
                onClick={() => onSelect(active ? null : id)}
                className={`flex w-full items-center gap-2 rounded px-2 py-1 text-left ${
                  active ? 'bg-blue-50!' : 'hover:bg-gray-50!'
                }`}
                style={{ paddingLeft: 8 + (depthOf[id] ?? 0) * 14 }}
              >
                <span
                  className="h-2 w-2 flex-none rounded-[2px]"
                  style={{ background: NODE_TYPE_COLOR[n.node_type] }}
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[13px] leading-5">
                    {n.title || '（未命名）'}
                  </span>
                  <span className="text-xs text-gray-400">
                    {NODE_TYPE_LABEL[n.node_type]} ·{' '}
                    {BLOCK_KIND_LABEL[specKindOf(n)]}
                  </span>
                </span>
              </button>
            );
          })}
        </Space>
      </div>

      {/* 右：编辑器 */}
      <div className="flex-1 overflow-auto rounded border border-gray-200 bg-white p-4">
        {!selNode ? (
          <div className="flex h-full items-center justify-center">
            <Empty description="在左侧点选一个组成部分，右侧声明其内容来源。" />
          </div>
        ) : (
          <Space direction="vertical" className="w-full!" size={12}>
            <div className="flex items-center justify-between gap-2">
              <Space wrap size={6}>
                <Typography.Text strong>{selNode.title || '（未命名）'}</Typography.Text>
                <Tag color={NODE_TYPE_COLOR[selNode.node_type]}>
                  {NODE_TYPE_LABEL[selNode.node_type]}
                </Tag>
              </Space>
              <Button type="primary" onClick={save}>
                保存内容声明
              </Button>
            </div>
            <Typography.Text type="secondary" className="block text-xs leading-5">
              内容声明决定这一处在文档里出现什么（写进节点 content_spec，不落业务数据）。装配引擎在「3.
              装配预览」按文档序逐节点把「内容」从项目数据（体系字段/表）/ 追踪图 / 执行清单算出来。
            </Typography.Text>
            <Divider className="my-1!" />

            <Select
              className="w-[280px]!"
              value={spec.kind}
              onChange={(kind: SpecKind) => patch({ kind })}
              options={KIND_OPTIONS}
              placeholder="选择这一处的内容类型"
            />

            {spec.kind === 'none' && (
              <Typography.Text type="secondary">
                无内容：该节点仅承担结构（如章节标题 / 占位）。通常章节类节点选此项。
              </Typography.Text>
            )}

            {spec.kind === 'text' && (
              <Space direction="vertical" className="w-full!" size={6}>
                <Typography.Text strong>文本套话</Typography.Text>
                <Typography.Text type="secondary" className="text-xs">
                  支持 {`{字段名}`} 占位符，装配时用项目数据标量字段展开（未填保持原文可见）。
                </Typography.Text>
                <Input.TextArea
                  rows={6}
                  value={spec.text}
                  onChange={(e) => patch({ text: e.target.value })}
                  placeholder="例如：本大纲依据 {依据…} 及被测软件 {软件名称} 的研制任务书编制……"
                />
              </Space>
            )}

            {spec.kind === 'field' && (
              <Space direction="vertical" className="w-full!" size={6}>
                <Typography.Text strong>项目字段</Typography.Text>
                <Space wrap>
                  <Input
                    className="w-[260px]!"
                    value={spec.field}
                    onChange={(e) => patch({ field: e.target.value })}
                    placeholder="字段名，如：项目代号 / 文档标识"
                  />
                  <Select
                    className="w-[220px]!"
                    allowClear
                    placeholder="doc_kind（缺省 = 跟随本文档类，再回退 project）"
                    value={spec.doc_kind}
                    onChange={(v?: string) => patch({ doc_kind: v })}
                    options={DOC_KIND_OPTIONS.map((v) => ({ value: v, label: v }))}
                  />
                </Space>
              </Space>
            )}

            {spec.kind === 'table' && (
              <Space direction="vertical" className="w-full!" size={6}>
                <Typography.Text strong>数据表（体系来源）</Typography.Text>
                <Select
                  className="w-[360px]!"
                  placeholder="选择体系里的表来源（下拉取全部体系自定义表并集）"
                  showSearch
                  value={spec.source}
                  onChange={(source?: string) =>
                    patch({ source, party_kind: undefined, category: undefined })
                  }
                  options={tableOptions}
                  notFoundContent={
                    tableOptions.length === 0 ? (
                      <span className="text-xs">暂无体系表——先到「台账编排」给体系加表</span>
                    ) : undefined
                  }
                />
                {spec.source && !tcol && (
                  <Typography.Text type="secondary" className="text-xs">
                    该表键在现有体系里未找到（可能在项目绑定的那套里才有）；装配按「项目数据」里同表键的表输出。
                  </Typography.Text>
                )}
                {spec.source && tcol?.hasPartyKind && (
                  <Input
                    className="w-[320px]!"
                    value={spec.party_kind ?? ''}
                    onChange={(e) => patch({ party_kind: e.target.value || undefined })}
                    placeholder="按「角色」列值筛选（可选），如：委托方 / 研制单位 / 测评机构"
                  />
                )}
                {spec.source && tcol?.hasCategory && (
                  <Input
                    className="w-[320px]!"
                    value={spec.category ?? ''}
                    onChange={(e) => patch({ category: e.target.value || undefined })}
                    placeholder="按「类别」列值筛选（可选），如：管理类文件 / 静态测评环境"
                  />
                )}
                {spec.source && tcol && !tcol.hasPartyKind && !tcol.hasCategory && (
                  <Typography.Text type="secondary" className="text-xs">
                    该表在体系里没有 party_kind / category 筛选列 → 整表输出。
                  </Typography.Text>
                )}
              </Space>
            )}

            {spec.kind === 'trace' && (
              <Space direction="vertical" className="w-full!" size={6}>
                <Typography.Text strong>追溯切片</Typography.Text>
                <Select
                  className="w-[320px]!"
                  value={spec.view ?? 'matrix'}
                  onChange={(view: string) => patch({ view })}
                  options={TRACE_VIEW_OPTIONS}
                />
              </Space>
            )}

            {spec.kind === 'slot' && (
              <Space direction="vertical" className="w-full!" size={6}>
                <Typography.Text strong>人工槽（结合执行清单）</Typography.Text>
                <Input
                  className="w-[280px]!"
                  value={spec.role}
                  onChange={(e) => patch({ role: e.target.value })}
                  placeholder="填写角色，如：大纲编写人员 / 测评组长"
                />
                <Input.TextArea
                  rows={4}
                  value={spec.placeholder}
                  onChange={(e) => patch({ placeholder: e.target.value })}
                  placeholder="未填写时的回落提示，如：请在此填写测评策略……"
                />
                <Typography.Text type="secondary" className="text-xs">
                  选「执行清单」的实例后，执行人员在该节点填的内容会装配进来；未填则回落此占位文案。
                </Typography.Text>
              </Space>
            )}
          </Space>
        )}
      </div>
    </div>
  );
}

/** 展示用：节点当前 content_spec 的 kind 标签（空/未知 → none）。 */
function specKindOf(n: TemplateNode): keyof typeof BLOCK_KIND_LABEL {
  const k = typeof n.content_spec?.kind === 'string' ? n.content_spec.kind : '';
  if (k === 'text' || k === 'field' || k === 'table' || k === 'trace' || k === 'slot') return k;
  return 'empty';
}
