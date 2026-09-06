import { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Empty, Select, Space, Spin, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { api } from '../../app/api';
import { errMsg, formatDateTime } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type {
  ExecutionInstance,
  ResolvedBlock,
  ResolvedDocument,
} from '../../app/types';
import { INSTANCE_STATUS_LABEL, LINK_TYPE_LABEL } from '../../app/types';
import { loadProjects } from '../project/projectSlice';

// 语义键 → 中文表头兜底（仅无 columns 推导时用；体系自定义列优先走装配返回的列头）
const KEY_LABEL: Record<string, string> = {
  party_kind: '角色',
  name: '名称',
  address: '地址',
  contact: '联系人',
  phone: '电话',
  category: '类别',
  seq: '序号',
  ident: '编号',
  version: '版本',
  pub_date: '发布日期',
  pub_org: '发布单位',
  phase: '阶段',
  time_range: '时间',
  location: '地点',
  device_type: '类型',
  safety_level: '安全级别',
  run_env: '运行环境',
  dev_env: '开发环境',
  language: '语言',
  code_scale: '代码规模',
  dev_org: '开发单位',
  resource_kind: '资源类型',
  version_config: '版本/配置',
  qty: '数量',
  note: '备注',
  provider: '提供方',
  external_ref: '编号',
  title: '名称/标题',
  module: '所属模块',
};

function short(id: string): string {
  return id.slice(0, 8);
}

function textOf(v: Record<string, unknown>): string {
  const t = v['text'];
  return typeof t === 'string' ? t : '';
}

function cellText(key: string, value: unknown): string {
  if (value === null || value === undefined) return '';
  void key;
  return String(value);
}

function tableData(rows: Array<Record<string, unknown>>) {
  return rows.map((r, i) => ({ __k: String(i), ...r }));
}

/** 装配返回的列头（体系自定义列）驱动渲染；无 columns 时退回按首行键推导。 */
function ColummedTable({
  columns,
  rows,
}: {
  columns: Array<{ key: string; label: string }>;
  rows: Array<Record<string, unknown>>;
}) {
  const cols: ColumnsType<Record<string, unknown>> = columns.map((c) => ({
    title: c.label || KEY_LABEL[c.key] || c.key,
    dataIndex: c.key,
    key: c.key,
    render: (v: unknown) => String(v ?? '') || '-',
  }));
  return (
    <Table<Record<string, unknown>>
      size="small"
      rowKey="__k"
      dataSource={tableData(rows)}
      columns={cols}
      pagination={false}
      scroll={{ x: Math.max(680, columns.length * 130) }}
    />
  );
}

/** 通用「数据表 / 测试项」渲染：从首行推导列。 */
function ScalarTable({ rows }: { rows: Array<Record<string, unknown>> }) {
  const keys: string[] = [];
  for (const r of rows) {
    for (const k of Object.keys(r)) if (!keys.includes(k)) keys.push(k);
  }
  const columns: ColumnsType<Record<string, unknown>> = keys.map((k) => ({
    title: KEY_LABEL[k] ?? k,
    dataIndex: k,
    key: k,
    render: (v: unknown) => cellText(k, v) || '-',
  }));
  return (
    <Table<Record<string, unknown>>
      size="small"
      rowKey="__k"
      dataSource={tableData(rows)}
      columns={columns}
      pagination={false}
      scroll={{ x: Math.max(680, keys.length * 130) }}
    />
  );
}

/** 追溯矩阵：需求一行，覆盖测试项并排。 */
function MatrixTable({ rows }: { rows: Array<Record<string, unknown>> }) {
  const nodeText = (n: unknown) => {
    if (!n || typeof n !== 'object') return '-';
    const o = n as Record<string, unknown>;
    return `${cellText('external_ref', o['external_ref']) || short(String(o['id'] ?? ''))} · ${
      typeof o['title'] === 'string' ? o['title'] : ''
    }`;
  };
  const columns: ColumnsType<Record<string, unknown>> = [
    {
      title: '需求（被测软件）',
      key: 'requirement',
      width: 300,
      render: (_, r) => (
        <div className="text-[13px] leading-5">
          <span className="font-medium">{nodeText(r['requirement'])}</span>
          {typeof (r['requirement'] as Record<string, unknown> | undefined)?.['module'] ===
            'string' && (
            <span className="ml-2 text-xs text-gray-400">
              {String((r['requirement'] as Record<string, unknown>)['module'])}
            </span>
          )}
        </div>
      ),
    },
    {
      title: '覆盖测试项',
      key: 'covering',
      render: (_, r) => {
        const arr = Array.isArray(r['covering']) ? r['covering'] : [];
        return (
          <Space size={[6, 6]} wrap>
            {arr.map((it, i) => {
              const o = (it ?? {}) as Record<string, unknown>;
              const link = typeof o['link_type'] === 'string' ? o['link_type'] : '';
              return (
                <Tag key={i} className="m-0!">
                  {nodeText(o)}
                  {link && (
                    <span className="ml-1 text-gray-400">
                      （{LINK_TYPE_LABEL[link as keyof typeof LINK_TYPE_LABEL] ?? link}）
                    </span>
                  )}
                </Tag>
              );
            })}
            {arr.length === 0 && <Typography.Text type="secondary">无覆盖项</Typography.Text>}
          </Space>
        );
      },
    },
  ];
  const data = rows.map((r, i) => ({ __k: String(i), ...r }));
  return (
    <Table<Record<string, unknown>>
      size="small"
      rowKey="__k"
      dataSource={data}
      columns={columns}
      pagination={false}
    />
  );
}

function BlockView({ block }: { block: ResolvedBlock }) {
  const { node_type, title, kind, value } = block;

  // 结构章节 → 标题
  if (node_type === 'chapter' || node_type === 'section' || node_type === 'appendix') {
    const level = Math.max(1, Math.min(4, block.depth + 1)) as 1 | 2 | 3 | 4;
    return (
      <Typography.Title level={level} className="mb-2! mt-0!">
        {title || '（未命名）'}
      </Typography.Title>
    );
  }
  // 封面/前置节点自身不成标题，其下的字段/段落子块会被逐条渲染
  if (node_type === 'frontmatter') return null;

  // 段落 / 字段：装配出的正文文本
  if ((kind === 'text' || kind === 'field') && textOf(value)) {
    return (
      <Typography.Paragraph className="mb-2! text-[13px] leading-6">
        {textOf(value)}
      </Typography.Paragraph>
    );
  }

  // 项目数据表（体系来源，列头来自装配返回的 columns）
  if (kind === 'table') {
    const rows = Array.isArray(value['rows']) ? (value['rows'] as Array<Record<string, unknown>>) : [];
    const cols = Array.isArray(value['columns'])
      ? (value['columns'] as Array<{ key: string; label: string }>)
      : null;
    return (
      <div className="mb-3!">
        {title && (
          <Typography.Text strong className="mb-1! block text-[13px]">
            {title}
          </Typography.Text>
        )}
        {rows.length === 0 ? (
          <Typography.Text type="secondary" className="text-xs">
            该项目暂无该表数据 → 到「执行清单」页为当前项目填写（值写项目级一份，装配即出现）。
          </Typography.Text>
        ) : cols && cols.length > 0 ? (
          <ColummedTable columns={cols} rows={rows} />
        ) : (
          <ScalarTable rows={rows} />
        )}
      </div>
    );
  }

  // 追溯切片
  if (kind === 'trace') {
    const view = typeof value['view'] === 'string' ? value['view'] : '';
    return (
      <div className="mb-3!">
        {title && (
          <Typography.Text strong className="mb-1! block text-[13px]">
            {title}
          </Typography.Text>
        )}
        {view === 'matrix' ? (
          <MatrixTable
            rows={Array.isArray(value['rows']) ? (value['rows'] as Array<Record<string, unknown>>) : []}
          />
        ) : view === 'test_items' ? (
          <ScalarTable rows={Array.isArray(value['items']) ? (value['items'] as Array<Record<string, unknown>>) : []} />
        ) : (
          <Alert type="warning" showIcon message={`未知追溯视图：${view}`} />
        )}
      </div>
    );
  }

  // 人工槽
  if (kind === 'slot') {
    const filled = value['filled'] === true;
    const body = typeof value['text'] === 'string' ? String(value['text']) : '';
    const role = typeof value['role'] === 'string' ? String(value['role']) : undefined;
    return (
      <div
        className={`mb-3! rounded border p-3! ${
          filled ? 'border-gray-300 bg-gray-50' : 'border-dashed border-orange-300 bg-orange-50'
        }`}
      >
        {title && (
          <Typography.Text strong className="mb-1! block text-[13px]">
            {title}
          </Typography.Text>
        )}
        {role && (
          <Tag className="mb-1!">{role}填写</Tag>
        )}
        {filled ? (
          <Typography.Paragraph className="mb-0! text-[13px] leading-6">{body}</Typography.Paragraph>
        ) : (
          <Typography.Text type="secondary" className="text-xs italic">
            （人工槽，待填写）{body}
          </Typography.Text>
        )}
      </div>
    );
  }

  // 图 / 其它占位节点 → 只出题注
  if (title) {
    return (
      <Typography.Text type="secondary" className="block text-xs">
        「{title}」（该类型内容渲染留后续）
      </Typography.Text>
    );
  }
  return null;
}

// ---------------------------------------------------------------------------

interface AssemblePreviewProps {
  templateId: string;
  templateName: string;
  templateKind: string;
}

export default function AssemblePreview({
  templateId,
  templateName,
  templateKind,
}: AssemblePreviewProps) {
  const dispatch = useAppDispatch();
  const { projects, currentProjectId } = useAppSelector((s) => s.projects);

  const [selProject, setSelProject] = useState<string | null>(null);
  const [instances, setInstances] = useState<ExecutionInstance[]>([]);
  const [selInstance, setSelInstance] = useState<string | null>(null);
  const [doc, setDoc] = useState<ResolvedDocument | null>(null);
  const [assembling, setAssembling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  // 项目列表兜底加载；默认取全局当前项目，否则第一个
  useEffect(() => {
    if (projects.length === 0) dispatch(loadProjects());
  }, [dispatch, projects.length]);
  useEffect(() => {
    if (selProject) return;
    const pick =
      projects.find((p) => p.id === currentProjectId)?.id ??
      projects[0]?.id ??
      null;
    setSelProject(pick);
  }, [projects, currentProjectId, selProject]);

  // 切项目 → 拉该项目的执行实例（人工槽取值用），并清掉上次选中的实例
  useEffect(() => {
    if (!selProject) {
      setInstances([]);
      setSelInstance(null);
      return;
    }
    setSelInstance(null);
    let cancelled = false;
    api
      .get<ExecutionInstance[]>(`/executions/instances?project_id=${selProject}`)
      .then((list) => {
        if (!cancelled) setInstances(list);
      })
      .catch(() => {
        /* 实例拉取失败不阻断预览；槽回落占位 */
      });
    return () => {
      cancelled = true;
    };
  }, [selProject]);

  // 装配（模板 + 项目 + 可选实例）
  useEffect(() => {
    if (!templateId || !selProject) {
      setDoc(null);
      return;
    }
    let cancelled = false;
    setAssembling(true);
    setError(null);
    const q =
      `/documents/templates/${templateId}/assemble?project_id=${selProject}` +
      (selInstance ? `&instance_id=${selInstance}` : '');
    api
      .get<ResolvedDocument>(q)
      .then((d) => {
        if (!cancelled) {
          setDoc(d);
          setAssembling(false);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setError(errMsg(e));
          setDoc(null);
          setAssembling(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [templateId, selProject, selInstance, reloadKey]);

  const sameTplInstances = useMemo(
    () => instances.filter((i) => i.template_id === templateId),
    [instances, templateId],
  );

  return (
    <Space direction="vertical" className="w-full!" size={10}>
      <Space wrap>
        <Select
          className="w-[240px]!"
          placeholder="选择装配所用的项目（项目数据/追踪数据）"
          value={selProject ?? undefined}
          onChange={(v) => setSelProject(v)}
          options={projects.map((p) => ({ value: p.id, label: p.name }))}
        />
        <Select
          className="w-[260px]!"
          allowClear
          placeholder="执行实例（可选；人工槽据此取值）"
          value={selInstance ?? undefined}
          onChange={(v) => setSelInstance(v ?? null)}
          options={sameTplInstances.map((i) => ({
            value: i.id,
            label: `${i.stage ?? '未分阶段'} · ${INSTANCE_STATUS_LABEL[i.status]} · ${formatDateTime(i.created_at)}`,
          }))}
          notFoundContent={
            sameTplInstances.length === 0 ? (
              <span className="text-xs">还没有本模板的执行实例 → 人工槽回落占位</span>
            ) : undefined
          }
        />
        <Button onClick={() => setReloadKey((k) => k + 1)} loading={assembling}>
          重新装配
        </Button>
      </Space>

      <Typography.Text type="secondary" className="text-xs">
        模板「{templateName}（{templateKind}）」按文档序装配为可预览正文：章节标题为结构节点；
        文本/字段来自项目数据（标量，取体系 doc_kind 作用域）；表格来自项目数据表（按体系自定义列出列头）；
        追溯切片现算追踪图；人工槽取执行清单填写内容。
      </Typography.Text>

      {error && <Alert type="error" showIcon message={error} />}

      <div className="rounded-lg border border-gray-200 bg-gray-100 p-4">
        {!selProject ? (
          <Empty description="请先在上方选择装配所用项目。" />
        ) : assembling && !doc ? (
          <div className="flex justify-center py-16">
            <Spin tip="装配中…" />
          </div>
        ) : doc ? (
          <div className="mx-auto max-w-[900px] rounded bg-white p-8 shadow-sm">
            <Space direction="vertical" className="w-full!" size={4}>
              {doc.blocks.map((b) => (
                <BlockView key={b.node_id} block={b} />
              ))}
            </Space>
          </div>
        ) : (
          <Empty description="装配结果将显示在这里。" />
        )}
      </div>
    </Space>
  );
}
