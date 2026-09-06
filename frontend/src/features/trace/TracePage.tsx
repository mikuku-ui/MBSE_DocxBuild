import { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  App,
  Button,
  Col,
  Descriptions,
  Divider,
  Drawer,
  Empty,
  Form,
  Input,
  Layout,
  List,
  Modal,
  Radio,
  Row,
  Space,
  Spin,
  Table,
  Tag,
  Tabs,
  Typography,
} from 'antd';
import { api } from '../../app/api';
import { errMsg } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import ErrorAlert from '../../components/ErrorAlert';
import NoProjectEmpty from '../../components/NoProjectEmpty';
import type {
  LinkType,
  TraceMatrix,
  TraceNode,
  TraceNodeKind,
} from '../../app/types';
import { LINK_TYPE_LABEL, NODE_KIND_LABEL } from '../../app/types';
import TraceCanvas from './TraceCanvas';
import type { SortOrderItem } from './layout';
import { KIND_COLOR, STAGE_ORDER, STAGE_META, stageSortKey } from './layout';
import { clearGraph, loadTraceGraph } from './traceSlice';

const { Sider, Content } = Layout;

// 侧栏/新增选项按「顶层在前」展示（需求 → … → 测试报告）
const DISPLAY_ORDER: TraceNodeKind[] = [...STAGE_ORDER].reverse();

type UploadRequirement = {
  id: string;
  title?: string;
  description?: string;
  module?: string;
  section?: string;
  attributes?: Record<string, unknown>;
};
type ParsedUpload = { source: string; requirements: UploadRequirement[] };
type UploadReport = { uploaded: number; skipped: string[] };

const SAMPLE_REQ_JSON = `{
  "source": "reqtool",
  "requirements": [
    {
      "id": "SR-1",
      "title": "支持账号密码登录",
      "description": "系统应提供账号+密码的登录方式，密码校验失败给出明确提示。",
      "module": "登录",
      "section": "3.2.1",
      "attributes": { "owner": "rd" }
    },
    {
      "id": "SR-2",
      "title": "支持第三方扫码登录",
      "description": "系统应支持通过扫码方式完成登录。",
      "module": "登录"
    }
  ]
}`;

const LINK_TYPE_HINT: Record<LinkType, string> = {
  derives_from: 'source 派生自 target（如 测试项 derived from 需求）',
  verifies: 'source 验证 target（下级验证上级是否满足）',
  satisfies: 'source 满足 target',
  refines: 'source 细化 target',
  part_of: 'source 是 target 的一部分（分解）',
};

interface EditorState {
  mode: 'create' | 'edit';
  node?: TraceNode;
}
interface EditorValues {
  title: string;
  module?: string;
  description?: string;
  section?: string;
  external_source?: string;
  external_ref?: string;
}

function genManualRef(): string {
  return `manual-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

function sectionOf(node: TraceNode): string {
  const v = node.attributes?.source_section;
  return typeof v === 'string' ? v : '';
}

export default function TracePage() {
  const dispatch = useAppDispatch();
  const { message, modal } = App.useApp();
  const { nodes, links, loading, error } = useAppSelector((s) => s.trace);
  const projectId = useAppSelector((s) => s.projects.currentProjectId);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [highlight, setHighlight] = useState<Set<string>>(new Set());
  const [matrixOpen, setMatrixOpen] = useState(false);
  const [matrix, setMatrix] = useState<TraceMatrix | null>(null);
  const [coverageOpen, setCoverageOpen] = useState(false);
  const [coverage, setCoverage] = useState<TraceNode[] | null>(null);
  const [reachOpen, setReachOpen] = useState(false);
  const [reach, setReach] = useState<TraceNode[]>([]);
  // 溯源/影响面各自独立 loading（点一个只转那一个，另一个保持可点/不转）
  const [reachBusy, setReachBusy] = useState<'up' | 'down' | null>(null);
  const [busy, setBusy] = useState(false);
  // 左栏当前选中的阶段 tab（展示该阶段节点列表）
  const [activeStage, setActiveStage] = useState<TraceNodeKind>('software_requirement');

  // 节点 新增/编辑
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [editorForm] = Form.useForm<EditorValues>();

  // 拖拽建边 → 选 link_type
  const [pendingLink, setPendingLink] = useState<{
    sourceId: string;
    targetId: string;
  } | null>(null);
  const [linkType, setLinkType] = useState<LinkType>('derives_from');

  // 需求 JSON 上传
  const [importOpen, setImportOpen] = useState(false);
  const [importText, setImportText] = useState('');
  const [importParsed, setImportParsed] = useState<ParsedUpload | null>(null);
  const [importErr, setImportErr] = useState<string | null>(null);

  // 当前项目作用域 query：所有 trace 请求都带 project_id（追踪数据按项目隔离）
  const pQuery = (extra = '') =>
    extra ? `?project_id=${projectId}&${extra}` : `?project_id=${projectId}`;

  // 切换项目 / 未选中项目时复位本页一次性状态（矩阵/覆盖/溯源抽屉、选中、高亮、编辑框）
  function resetTransient() {
    setSelectedId(null);
    setHighlight(new Set());
    setMatrixOpen(false);
    setMatrix(null);
    setCoverageOpen(false);
    setCoverage(null);
    setReachOpen(false);
    setReach([]);
    setReachBusy(null);
    setBusy(false);
    setEditor(null);
    setPendingLink(null);
    setImportOpen(false);
    setImportText('');
    setImportParsed(null);
    setImportErr(null);
  }

  // 项目选中即拉取其追踪图；项目变化时先复位/清空上一项目图再加载
  // （本页数据 100% 属于当前项目，切换项目不得残留上一项目的图）
  useEffect(() => {
    resetTransient();
    dispatch(clearGraph());
    if (projectId) {
      dispatch(loadTraceGraph(projectId));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dispatch, projectId]);

  const selected = nodes.find((n) => n.id === selectedId) ?? null;
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);

  // 在画布上点节点 → 打开右侧「内容」面板（纯查看）；点别处关闭并清高亮。
  function openDetail(id: string | null) {
    setSelectedId(id);
    setHighlight(new Set());
  }
  // 点左侧列表 → 只把对应节点在画布上高亮，不同步打开右侧面板。
  function focusNode(id: string) {
    setSelectedId(null);
    setHighlight(new Set([id]));
  }

  async function openMatrix() {
    if (!projectId) return;
    setMatrixOpen(true);
    if (matrix) return;
    try {
      setMatrix(await api.get<TraceMatrix>(`/trace/matrix${pQuery()}`));
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function openCoverage() {
    if (!projectId) return;
    setCoverageOpen(true);
    if (coverage) return;
    try {
      setCoverage(await api.get<TraceNode[]>(`/trace/coverage${pQuery()}`));
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  // 向上追溯 / 向下影响面：各自独立 loading；结果用「无遮罩」抽屉承载，
  // 收起右侧内容面板，让画布上的高亮链不被遮住。
  async function runReach(node: TraceNode, dir: 'up' | 'down') {
    if (!projectId) return;
    setReachBusy(dir);
    try {
      const arr = await api.get<TraceNode[]>(
        `/trace/nodes/${node.id}/reachable${pQuery(`direction=${dir}`)}`,
      );
      setReach(arr);
      setSelectedId(null); // 收起右侧内容面板，避免叠加遮挡画布高亮
      setHighlight(new Set([node.id, ...arr.map((n) => n.id)]));
      setReachOpen(true);
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setReachBusy(null);
    }
  }

  // ---- 节点增删改 ----
  function openCreate() {
    setEditor({ mode: 'create' }); // 追踪页只新增需求节点
  }
  // 节点模型约定：仅需求节点在本页可编辑；测试项/用例/记录/报告内容由其所属模块维护。
  function openEdit(node: TraceNode) {
    if (node.kind !== 'software_requirement') {
      message.info('仅需求节点可在本页编辑；测试项/用例/记录/报告的内容由对应模块维护。');
      return;
    }
    setEditor({ mode: 'edit', node });
  }

  async function saveEditor(v: EditorValues) {
    if (!projectId) return;
    setBusy(true);
    try {
      const payloadModule = v.module?.trim() || undefined;
      if (editor?.mode === 'edit' && editor.node) {
        const id = editor.node.id;
        const attrs = { ...(editor.node.attributes as Record<string, unknown>) };
        if (v.section?.trim()) attrs.source_section = v.section.trim();
        else delete attrs.source_section;
        await api.put(`/trace/nodes/${id}${pQuery()}`, {
          kind: editor.node.kind, // 编辑不迁移阶段（仅需求可编辑，阶段锁死）
          title: v.title.trim(),
          module: payloadModule,
          description: v.description?.trim() || undefined,
          attributes: attrs,
        });
        message.success('节点已保存');
      } else {
        const ref = v.external_ref?.trim() || genManualRef();
        const created = await api.post<TraceNode>(`/trace/nodes${pQuery()}`, {
          kind: 'software_requirement', // 追踪页只新增需求节点
          external_source: v.external_source?.trim() || 'manual',
          external_ref: ref,
          module: payloadModule,
          title: v.title.trim(),
          description: v.description?.trim() || undefined,
        });
        if (v.section?.trim()) {
          await api.put(`/trace/nodes/${created.id}${pQuery()}`, {
            attributes: { source_section: v.section.trim() },
          });
        }
        message.success(`已新增「${v.title.trim()}」`);
      }
      setEditor(null);
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  function confirmDeleteNode(node: TraceNode) {
    modal.confirm({
      title: `删除节点「${node.external_ref} · ${node.title}」？`,
      content: '会一并删除与该节点相连的追溯边（数据库级联）。',
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        if (!projectId) return;
        try {
          await api.del(`/trace/nodes/${node.id}${pQuery()}`);
          message.success('节点已删除');
          if (selectedId === node.id) openDetail(null);
          dispatch(loadTraceGraph(projectId));
        } catch (e) {
          message.error(errMsg(e));
        }
      },
    });
  }

  // ---- 建边 / 删边 ----
  function handleConnectEdge(sourceId: string, targetId: string) {
    setPendingLink({ sourceId, targetId });
    setLinkType('derives_from');
  }

  async function confirmLink() {
    if (!pendingLink || !projectId) return;
    setBusy(true);
    try {
      await api.post(`/trace/links${pQuery()}`, {
        source_node_id: pendingLink.sourceId,
        target_node_id: pendingLink.targetId,
        link_type: linkType,
      });
      const s = byId.get(pendingLink.sourceId);
      const t = byId.get(pendingLink.targetId);
      message.success(`已建立追溯边：${s?.external_ref ?? '?'} → ${t?.external_ref ?? '?'}`);
      setPendingLink(null);
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  async function removeLink(linkId: string) {
    if (!projectId) return;
    try {
      await api.del(`/trace/links/${linkId}${pQuery()}`);
      message.success('追溯边已删除');
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  // ---- 需求 JSON 上传 ----
  function resetImport() {
    setImportText('');
    setImportParsed(null);
    setImportErr(null);
  }
  function openImport() {
    resetImport();
    setImportOpen(true);
  }
  function parseImport() {
    setImportErr(null);
    setImportParsed(null);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw: any = JSON.parse(importText);
      if (!raw || typeof raw !== 'object' || typeof raw.source !== 'string' || raw.source.trim() === '') {
        throw new Error('顶层需要非空字符串字段 source（研发系统标识）');
      }
      if (!Array.isArray(raw.requirements)) {
        throw new Error('顶层需要数组字段 requirements');
      }
      for (const r of raw.requirements) {
        if (!r || typeof r.id !== 'string' || r.id.trim() === '') {
          throw new Error('requirements 每项需要非空字符串 id（来源侧唯一键）');
        }
      }
      setImportParsed({ source: raw.source.trim(), requirements: raw.requirements });
    } catch (e) {
      setImportErr(e instanceof Error ? e.message : String(e));
    }
  }
  async function doUpload() {
    if (!importParsed || !projectId) return;
    setBusy(true);
    try {
      const rep = await api.post<UploadReport>(`/trace/requirements/upload${pQuery()}`, importParsed);
      const extra = rep.skipped.length > 0 ? `，跳过 ${rep.skipped.length} 条` : '';
      message.success(`已导入 ${rep.uploaded} 条需求${extra}（重复 source+id 已覆盖更新）`);
      setImportOpen(false);
      resetImport();
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setBusy(false);
    }
  }

  // ---- 分组与排序 ----
  const byKind = Object.fromEntries(
    STAGE_ORDER.map((k) => [k, [] as TraceNode[]]),
  ) as Record<TraceNodeKind, TraceNode[]>;
  for (const n of nodes) byKind[n.kind]?.push(n);
  // 每阶段节点按 sort_key（用户顺序，落库）排，画布与左栏同源
  const byKindSorted = Object.fromEntries(
    STAGE_ORDER.map((k) => [k, [...byKind[k]].sort(stageSortKey)]),
  ) as Record<TraceNodeKind, TraceNode[]>;

  // 提交整列顺序（画布拖拽松手 / 左栏上移下移 共用）：重编号 0..n-1 → PUT 落库
  async function applyReorder(orders: SortOrderItem[]) {
    if (!projectId || orders.length === 0) return;
    setBusy(true);
    try {
      await api.put(`/trace/nodes/order${pQuery()}`, { orders });
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(`排序保存失败：${errMsg(e)}`);
    } finally {
      setBusy(false);
    }
  }

  function moveNode(kind: TraceNodeKind, id: string, dir: 'up' | 'down') {
    const arr = byKindSorted[kind];
    const i = arr.findIndex((n) => n.id === id);
    const j = i + (dir === 'up' ? -1 : 1);
    if (i < 0 || j < 0 || j >= arr.length) return;
    const next = [...arr];
    const [moved] = next.splice(i, 1);
    next.splice(j, 0, moved);
    void applyReorder(next.map((n, idx) => ({ id: n.id, sort_key: idx })));
  }

  // 「整理」按钮：按需求锚定规则一键重排全图（需求越靠前，其派生链上的各阶段越靠前）
  async function arrangeAll() {
    if (!projectId) return;
    setBusy(true);
    try {
      await api.post(`/trace/nodes/arrange${pQuery()}`);
      message.success('已按需求顺序整理');
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(`整理失败：${errMsg(e)}`);
    } finally {
      setBusy(false);
    }
  }

  const editorInitial = useMemo<EditorValues | undefined>(() => {
    if (!editor) return undefined;
    if (editor.mode === 'create') {
      return {
        title: '',
        external_source: 'manual',
      };
    }
    const n = editor.node!;
    return {
      title: n.title,
      module: n.module ?? '',
      description: n.description ?? '',
      section: sectionOf(n),
    };
  }, [editor]);

  const pendingEdgeLabels = (() => {
    if (!pendingLink) return null;
    const s = byId.get(pendingLink.sourceId);
    const t = byId.get(pendingLink.targetId);
    return {
      src: `${s?.external_ref ?? '?'} · ${s?.title ?? ''}`,
      tgt: `${t?.external_ref ?? '?'} · ${t?.title ?? ''}`,
    };
  })();

  // 未选项目时不发任何 trace 请求：先引导新建/选择项目。
  if (!projectId) {
    return (
      <NoProjectEmpty center description="尚未选择项目">
        <Typography.Text type="secondary">
          请在右上角新建或选择一个项目；每个项目拥有一套独立的追踪数据。
        </Typography.Text>
      </NoProjectEmpty>
    );
  }

  return (
    <Layout className="h-[calc(100vh-128px)]">
      <Sider
        width={372}
        theme="light"
        className="rail-sider h-full! overflow-hidden!"
      >
        {/* 顶部固定区：不进滚动 */}
        <div className="flex-none">
          <div className="flex items-center justify-between">
            <Typography.Text strong>需求追踪图</Typography.Text>
            <Button
              size="small"
              type="text"
              loading={busy}
              onClick={() => projectId && dispatch(loadTraceGraph(projectId))}
            >
              刷新
            </Button>
          </div>
          {/* 操作按钮：一整行排下 */}
          <div className="mt-1 flex gap-1.5">
            <Button size="small" type="primary" className="flex-[2]" onClick={() => openCreate()}>
              ＋ 新增
            </Button>
            <Button size="small" className="flex-[3]" onClick={openImport}>
              导入 JSON
            </Button>
            <Button size="small" className="flex-[2]" onClick={openMatrix}>
              矩阵
            </Button>
            <Button size="small" className="flex-[2]" onClick={openCoverage}>
              覆盖
            </Button>
          </div>
          <ErrorAlert error={error} className="mt-2" />
          <Typography.Paragraph type="secondary" className="mb-0! mt-1.5! text-xs!">
            各阶段内容分别在对应模块维护：本页仅录入<strong>需求</strong>节点，
            其余阶段（测试项/用例/记录/报告）在画布上查看 / 连线 / 排序 / 删除。
          </Typography.Paragraph>
          <Divider className="mt-2! mb-1!" />
        </div>

        {/* 下方：分阶段 tab；仅节点列表区域滚动 */}
        <div className="min-h-0 flex-1">
          <Tabs
            className="rail-tabs"
            size="small"
            activeKey={activeStage}
            onChange={(k) => setActiveStage(k as TraceNodeKind)}
            tabBarStyle={{ marginBottom: 2 }}
            items={DISPLAY_ORDER.map((kind) => {
              const list = byKindSorted[kind];
              return {
                key: kind,
                label: (
                  <span>
                    <span style={{ color: KIND_COLOR[kind], fontWeight: 600 }}>{NODE_KIND_LABEL[kind]}</span>
                    <Typography.Text type="secondary" className="ml-[3px] text-xs">
                      {list.length}
                    </Typography.Text>
                  </span>
                ),
                children: (
                  <div className="flex h-full flex-col">
                    <div className="flex-none">
                      <Typography.Text type="secondary" className="text-xs">
                        {STAGE_META[kind].subline}
                      </Typography.Text>
                    </div>
                    <div className="mt-1 min-h-0 flex-1 overflow-y-auto">
                      {list.length === 0 ? (
                        <Empty
                          image={Empty.PRESENTED_IMAGE_SIMPLE}
                          description={
                            kind === 'software_requirement'
                              ? '本阶段暂无需求，点上方「＋ 新增」录入'
                              : '本阶段暂无节点（由对应模块/导入产生）'
                          }
                        />
                      ) : (
                        <List
                          size="small"
                          dataSource={list}
                          renderItem={(n, index) => (
                            <List.Item className="px-0! py-[3px]!">
                              <Space.Compact className="w-full!">
                                <Button
                                  size="small"
                                  className="text-[11px]!"
                                  disabled={index === 0}
                                  title="上移（顺序会保存）"
                                  onClick={() => moveNode(kind, n.id, 'up')}
                                >
                                  ▲
                                </Button>
                                <Button
                                  size="small"
                                  className="text-[11px]!"
                                  disabled={index === list.length - 1}
                                  title="下移（顺序会保存）"
                                  onClick={() => moveNode(kind, n.id, 'down')}
                                >
                                  ▼
                                </Button>
                                <Button
                                  type={
                                    selectedId === n.id || highlight.has(n.id)
                                      ? 'primary'
                                      : 'text'
                                  }
                                  size="small"
                                  block
                                  className="overflow-hidden! text-left!"
                                  title={`${n.external_ref} · ${n.title}${n.description ? `\n${n.description}` : ''}`}
                                  onClick={() => focusNode(n.id)}
                                >
                                  {n.external_ref} · {n.title}
                                </Button>
                                {n.kind === 'software_requirement' && (
                                  <Button size="small" onClick={() => openEdit(n)}>
                                    编辑
                                  </Button>
                                )}
                              </Space.Compact>
                            </List.Item>
                          )}
                        />
                      )}
                    </div>
                  </div>
                ),
              };
            })}
          />
        </div>
      </Sider>

      <Content className="relative">
        {loading ? (
          <div className="flex h-full items-center justify-center">
            <Spin size="large" />
          </div>
        ) : (
          // 画布常挂载：空项目也渲染五条空泳道（泳道背景与节点无关），fitView 首次框入全部
          <div className="absolute inset-0">
            <TraceCanvas
              nodes={nodes}
              links={links}
              selectedId={selectedId}
              highlight={highlight}
              onSelectNode={openDetail}
              onEditNode={(id) => {
                const n = byId.get(id);
                if (n) openEdit(n);
              }}
              onDeleteNode={(node) => confirmDeleteNode(node)}
              onReachNode={runReach}
              reachBusy={reachBusy}
              onConnectEdge={handleConnectEdge}
              onDeleteLink={removeLink}
              onReorder={applyReorder}
            />
            {nodes.length === 0 && (
              <div className="pointer-events-none absolute left-1/2 top-3.5 z-20 -translate-x-1/2 rounded-lg bg-white px-3 py-1 shadow-[0_2px_8px_rgba(0,0,0,0.12)]">
                <Typography.Text type="secondary" className="text-xs">
                  当前项目为空：点左栏「＋ 新增」录入需求，或「导入需求 JSON」。
                </Typography.Text>
              </div>
            )}
            <Button
              size="small"
              className="absolute! right-3! top-2! z-30"
              loading={busy}
              onClick={arrangeAll}
              title="按需求顺序一键整理：需求越靠前，其派生链上的各阶段节点越靠前"
            >
              整理
            </Button>
          </div>
        )}
      </Content>

      {/* 节点新增/编辑 */}
      <Modal
        key={editor ? `${editor.mode}:${editor.node?.id ?? 'new'}` : 'none'}
        title={editor?.mode === 'edit' ? '编辑需求' : '新增需求'}
        open={editor !== null}
        onCancel={() => setEditor(null)}
        width={560}
        okText={editor?.mode === 'edit' ? '保存' : '新增'}
        cancelText="取消"
        okButtonProps={{ loading: busy }}
        onOk={() => editorForm.submit()}
        destroyOnClose
      >
        {editor && (
          <Form
            form={editorForm}
            layout="vertical"
            initialValues={editorInitial}
            onFinish={saveEditor}
          >
            {editor.mode === 'create' ? (
              <Alert
                type="info"
                showIcon
                className="mb-2!"
                message="仅新增需求节点（阶段固定为「需求」）"
                description="测试项 / 用例 / 记录 / 报告由对应模块或研发系统维护，不在本页新增；创建后可在画布上与各阶段节点建立追溯边。"
              />
            ) : (
              <Alert
                type="info"
                showIcon
                className="mb-2!"
                message="仅编辑需求节点（阶段固定为「需求」）"
                description="测试项 / 用例 / 记录 / 报告的内容由对应模块或研发系统维护，不在本页编辑；此表单仅改需求的标题 / 模块 / 章节号 / 内容。"
              />
            )}
            <Row gutter={12}>
              <Col span={24}>
                <Form.Item
                  label="标题"
                  name="title"
                  rules={[{ required: true, message: '请填写标题' }]}
                  className="mb-3!"
                >
                  <Input placeholder="如：3.2.1 支持账号密码登录" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item label="模块（可选）" name="module" className="mb-3!">
                  <Input placeholder="如：登录" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item
                  label="章节号（可选，存入 source_section）"
                  name="section"
                  className="mb-3!"
                >
                  <Input placeholder="如：3.2.1" />
                </Form.Item>
              </Col>
              {editor.mode === 'create' && (
                <>
                  <Col span={12}>
                    <Form.Item
                      label="来源 external_source（默认 manual）"
                      name="external_source"
                      className="mb-3!"
                    >
                      <Input placeholder="manual / 研发系统名" />
                    </Form.Item>
                  </Col>
                  <Col span={12}>
                    <Form.Item
                      label="external_ref（留空自动生成）"
                      name="external_ref"
                      className="mb-3!"
                    >
                      <Input placeholder="留空自动生成" />
                    </Form.Item>
                  </Col>
                </>
              )}
            </Row>
            <Form.Item label="内容 / 描述" name="description" className="mb-0!">
              <Input.TextArea rows={2} placeholder="需求/阶段内容描述…" />
            </Form.Item>
          </Form>
        )}
      </Modal>

      {/* 建边：选 link_type */}
      <Modal
        title="建立追溯边（选择链接类型）"
        open={pendingLink !== null}
        onCancel={() => setPendingLink(null)}
        width={520}
        okText="建立"
        cancelText="取消"
        okButtonProps={{ loading: busy }}
        onOk={confirmLink}
      >
        {pendingEdgeLabels && (
          <>
            <Typography.Paragraph className="text-[13px]">
              从（低层/覆盖方）<Typography.Text strong>{pendingEdgeLabels.src}</Typography.Text>
              <br />
              到（高层/被覆盖方）<Typography.Text strong>{pendingEdgeLabels.tgt}</Typography.Text>
            </Typography.Paragraph>
            <Radio.Group
              onChange={(e) => setLinkType(e.target.value as LinkType)}
              value={linkType}
              className="w-full"
            >
              <Space direction="vertical" className="w-full!">
                {(Object.keys(LINK_TYPE_LABEL) as LinkType[]).map((lt) => (
                  <Radio key={lt} value={lt} className="w-full!">
                    <Space direction="vertical" size={0}>
                      <span>{LINK_TYPE_LABEL[lt]}</span>
                      <Typography.Text type="secondary" className="text-xs">
                        {LINK_TYPE_HINT[lt]}
                      </Typography.Text>
                    </Space>
                  </Radio>
                ))}
              </Space>
            </Radio.Group>
          </>
        )}
      </Modal>

      {/* 选中节点详情 */}
      <Drawer
        title="节点详情（只读）"
        open={selected !== null}
        onClose={() => openDetail(null)}
        width={440}
      >
        {selected && (
          <>
            <Descriptions column={1} bordered size="small">
              <Descriptions.Item label="阶段">
                <Tag color={KIND_COLOR[selected.kind]}>{NODE_KIND_LABEL[selected.kind]}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="标题">{selected.title}</Descriptions.Item>
              <Descriptions.Item label="标识">{selected.external_ref}</Descriptions.Item>
              <Descriptions.Item label="来源">{selected.external_source}</Descriptions.Item>
              <Descriptions.Item label="模块">{selected.module ?? '-'}</Descriptions.Item>
              {sectionOf(selected) && (
                <Descriptions.Item label="章节号">{sectionOf(selected)}</Descriptions.Item>
              )}
              <Descriptions.Item label="描述">{selected.description ?? '-'}</Descriptions.Item>
            </Descriptions>
            <Divider className="my-4!" />
            <Typography.Paragraph type="secondary" className="mt-0! text-xs">
              本面板只读展示节点内容；对节点执行操作请在画布上<strong>右键</strong>该节点
              （向上追溯 / 向下影响面 / 删除；编辑仅对需求节点开放，也可在左栏该行的「编辑」进入）。
            </Typography.Paragraph>
            <Typography.Paragraph type="secondary" className="text-xs">
              拖动节点排序：在本阶段列内上下拖到目标位置、松手即保存顺序（刷新不丢）；也可用
              左侧列表的 ↑/↓ 微调。点右上角「整理」按需求顺序一键重排——需求越靠前，其派生
              链上的测试项/用例/记录/报告越靠前。建立追踪边：从节点右侧端点拖到另一节点左侧
              端点，会弹出选择链接类型。左侧=下级/覆盖方，右侧=上级/被覆盖方。
            </Typography.Paragraph>
          </>
        )}
      </Drawer>

      {/* 追溯矩阵 */}
      <Drawer
        title="追溯矩阵（需求 × 覆盖项）"
        open={matrixOpen}
        onClose={() => setMatrixOpen(false)}
        width={820}
      >
        {matrix ? (
          <Table
            size="small"
            rowKey={(r) => r.requirement.id}
            dataSource={matrix.rows}
            pagination={{ pageSize: 12 }}
            columns={[
              {
                title: '需求标识',
                dataIndex: ['requirement', 'external_ref'],
                width: 120,
              },
              { title: '标题', dataIndex: ['requirement', 'title'], ellipsis: true },
              {
                title: '模块',
                dataIndex: ['requirement', 'module'],
                width: 110,
                render: (v: string | null) => v ?? '-',
              },
              {
                title: '覆盖项（来源 · 链接）',
                key: 'covering',
                render: (_, row) =>
                  row.covering.length === 0 ? (
                    <Tag color="red">未覆盖</Tag>
                  ) : (
                    row.covering.map(([n, lt]) => (
                      <Tag key={n.id}>
                        {n.external_ref} · {LINK_TYPE_LABEL[lt]}
                      </Tag>
                    ))
                  ),
              },
            ]}
          />
        ) : (
          <Empty description="加载中…" />
        )}
      </Drawer>

      {/* 覆盖分析 */}
      <Drawer
        title="覆盖分析：缺失 trace 的需求"
        open={coverageOpen}
        onClose={() => setCoverageOpen(false)}
        width={560}
      >
        {coverage ? (
          <Table
            size="small"
            rowKey="id"
            dataSource={coverage}
            pagination={{ pageSize: 10 }}
            columns={[
              { title: '需求标识', dataIndex: 'external_ref', width: 120 },
              { title: '标题', dataIndex: 'title', ellipsis: true },
              { title: '状态', width: 90, render: () => <Tag color="red">未覆盖</Tag> },
            ]}
          />
        ) : (
          <Empty description="加载中…" />
        )}
      </Drawer>

      {/* 可达集合：无遮罩，让画布上的高亮链保持可见 */}
      <Drawer
        title="可达节点集合"
        open={reachOpen}
        onClose={() => setReachOpen(false)}
        width={560}
        mask={false}
      >
        <List
          size="small"
          dataSource={reach}
          renderItem={(n) => (
            <List.Item>
              <Space>
                <Tag color={KIND_COLOR[n.kind]}>{NODE_KIND_LABEL[n.kind]}</Tag>
                {n.external_ref} · {n.title}
              </Space>
            </List.Item>
          )}
        />
        {reach.length === 0 && <Empty description="无可达节点" />}
        <Button block className="mt-3" onClick={() => setReachOpen(false)}>
          关闭（高亮保留在画布）
        </Button>
      </Drawer>

      {/* 需求 JSON 上传 */}
      <Modal
        title="导入需求 JSON"
        open={importOpen}
        onCancel={() => setImportOpen(false)}
        width={680}
        okText="导入"
        cancelText="取消"
        okButtonProps={{ disabled: !importParsed, loading: busy }}
        onOk={doUpload}
      >
        <Alert
          type="info"
          showIcon
          className="mb-2!"
          message="需求节点模型约定"
          description="每条需求只约定通用要素：id（↔ 外部唯一键）、title（标题，可自带“章节号+标题”）、description（内容）、module（软分组）。章节号等来源专有格式不强约束——可填 section（透传 attributes.source_section）或任意 attributes。"
        />
        <Space className="mb-2!">
          <Button
            size="small"
            onClick={() => {
              setImportText(SAMPLE_REQ_JSON);
              setImportParsed(null);
              setImportErr(null);
            }}
          >
            填入示例
          </Button>
          <Typography.Text type="secondary">研发系统导出的需求列表 JSON</Typography.Text>
        </Space>
        <Input.TextArea
          rows={9}
          spellCheck={false}
          className="font-[monospace]! text-xs!"
          value={importText}
          placeholder='{"source":"reqtool","requirements":[{"id":"SR-1","title":"…","description":"…","module":"登录"}]}'
          onChange={(e) => {
            setImportText(e.target.value);
            setImportParsed(null);
            setImportErr(null);
          }}
        />
        {importErr && <Alert type="error" showIcon className="mt-2" message={importErr} />}
        {importParsed && (
          <div className="mt-2">
            <Typography.Paragraph className="mb-1!">
              解析成功：source = <Typography.Text code>{importParsed.source}</Typography.Text>，共{' '}
              <Typography.Text strong>{importParsed.requirements.length}</Typography.Text> 条需求
              （重复 source + id 会覆盖更新，不漂移节点 id）。
            </Typography.Paragraph>
            <List
              size="small"
              dataSource={importParsed.requirements}
              renderItem={(r) => (
                <List.Item className="px-0! py-0.5!">
                  <Typography.Text type="secondary" className="mr-2">
                    {r.id}
                  </Typography.Text>
                  {r.title?.trim() || <Typography.Text type="secondary">（未填标题，将回退为 id）</Typography.Text>}
                </List.Item>
              )}
            />
          </div>
        )}
      </Modal>
    </Layout>
  );
}
