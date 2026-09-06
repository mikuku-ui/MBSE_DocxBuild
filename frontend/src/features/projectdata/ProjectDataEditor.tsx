// 项目数据编辑器（「全量可看可改」的填数面，被项目页与执行清单共用）。
//
// 数据视图自描述（结构 × 值）→ 这里按视图直接渲染：标量字段按 doc_kind 分组，
// 每张表按 scheme 给的自定义列渲染可增删行。改动先留在内存，点「保存数据」
// 一次性整存替换 PUT /api/projects/{id}/data（服务端裁剪到体系列、重排 row_index）。
//
// 使用方：ProjectPage 展开行（项目管理=全量可看可改）、ExecutionPage 抽屉
// （执行清单 = 填数面）。两者写入的是同一份「项目级」值。

import { useCallback, useEffect, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Divider,
  Empty,
  Input,
  Popconfirm,
  Select,
  Space,
  Spin,
  Table,
  Tag,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { api } from '../../app/api';
import { errMsg } from '../../app/format';
import type {
  DataColumn,
  ProjectDataView,
  ProjectDataWrite,
} from '../../app/types';
import { DOC_KIND_OPTIONS } from '../../app/types';

// ---------------------------------------------------------------------------
// 编辑期模型：把视图（带 row_index）转成以 rid 为键的可变行，保存时再回写。
// ---------------------------------------------------------------------------

interface MField {
  id: string;
  doc_kind: string;
  field_key: string;
  label: string;
  value: string;
}

interface MRow {
  rid: string;
  cells: Record<string, string>;
}

interface MTable {
  id: string;
  table_key: string;
  label: string;
  columns: DataColumn[];
  rows: MRow[];
}

interface MModel {
  project_id: string;
  scheme_id: string | null;
  fields: MField[];
  tables: MTable[];
}

function uid(): string {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `r-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function toModel(view: ProjectDataView): MModel {
  return {
    project_id: view.project_id,
    scheme_id: view.scheme_id,
    fields: view.fields.map((f) => ({ ...f, value: f.value ?? '' })),
    tables: view.tables.map((t) => ({
      id: t.id,
      table_key: t.table_key,
      label: t.label || t.table_key,
      columns: t.columns,
      rows: t.rows.map((r) => ({ rid: uid(), cells: { ...r.cells } })),
    })),
  };
}

/** 把编辑期模型 → PUT body（trim、丢空行/空单元；服务端还会裁剪列白名单并重排 row_index）。 */
function toWrite(m: MModel): ProjectDataWrite {
  return {
    fields: m.fields.map((f) => ({ field_id: f.id, value: f.value.trim() })),
    tables: m.tables.map((t) => {
      const rows: Array<{ row_index: number; cells: Record<string, string> }> = [];
      for (const r of t.rows) {
        const cells: Record<string, string> = {};
        for (const c of t.columns) {
          const v = (r.cells[c.column_key] ?? '').trim();
          if (v) cells[c.column_key] = v;
        }
        if (Object.keys(cells).length > 0) rows.push({ row_index: rows.length, cells });
      }
      return { table_id: t.id, rows };
    }),
  };
}

const trim = (s: string): string => s.trim();

// ---------------------------------------------------------------------------
// 字段分组渲染（project 在前，其余 doc_kind 保持体系内定义顺序）
// ---------------------------------------------------------------------------

function docKindLabel(dk: string): string {
  return dk === 'project' ? 'project（项目共享）' : dk;
}

interface ProjectDataEditorProps {
  projectId: string;
  /** 嵌入窄容器（执行清单抽屉）时换紧凑文案。 */
  compact?: boolean;
}

export default function ProjectDataEditor({ projectId, compact }: ProjectDataEditorProps) {
  const [model, setModel] = useState<MModel | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const view = await api.get<ProjectDataView>(`/projects/${projectId}/data`);
      setModel(toModel(view));
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    setModel(null);
    void load();
  }, [load]);

  async function save() {
    if (!model) return;
    setSaving(true);
    setError(null);
    try {
      const view = await api.put<ProjectDataView>(
        `/projects/${projectId}/data`,
        toWrite(model),
      );
      setModel(toModel(view));
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  const patchField = (id: string, value: string) =>
    setModel((m) =>
      m ? { ...m, fields: m.fields.map((f) => (f.id === id ? { ...f, value } : f)) } : m,
    );

  const patchCell = (tableId: string, rid: string, colKey: string, value: string) =>
    setModel((m) =>
      m
        ? {
            ...m,
            tables: m.tables.map((t) =>
              t.id === tableId
                ? {
                    ...t,
                    rows: t.rows.map((r) =>
                      r.rid === rid
                        ? { ...r, cells: { ...r.cells, [colKey]: value } }
                        : r,
                    ),
                  }
                : t,
            ),
          }
        : m,
    );

  const addRow = (tableId: string) =>
    setModel((m) =>
      m
        ? {
            ...m,
            tables: m.tables.map((t) =>
              t.id === tableId ? { ...t, rows: [...t.rows, { rid: uid(), cells: {} }] } : t,
            ),
          }
        : m,
    );

  const delRow = (tableId: string, rid: string) =>
    setModel((m) =>
      m
        ? {
            ...m,
            tables: m.tables.map((t) =>
              t.id === tableId ? { ...t, rows: t.rows.filter((r) => r.rid !== rid) } : t,
            ),
          }
        : m,
    );

  const countNonEmptyFields = model?.fields.filter((f) => trim(f.value)).length ?? 0;
  const countTableRows = (t: MTable) =>
    t.rows.filter((r) => Object.values(r.cells).some((v) => trim(v))).length;

  if (loading && !model) {
    return (
      <div className="flex justify-center py-6">
        <Spin tip="加载项目数据…" />
      </div>
    );
  }

  if (!model) {
    return error ? <Alert type="error" showIcon message={error} /> : null;
  }

  if (!model.scheme_id) {
    return (
      <Alert
        type="info"
        showIcon
        message={
          compact
            ? '该项目尚未绑定体系（执行方式），暂无数据可填。'
            : '该项目尚未绑定体系（执行方式）。'
        }
        description={
          compact
            ? '到「项目管理」页为该项目的「体系」列选择一套执行方式后，这里才有可填的结构。'
            : '在下方表格「体系（执行方式）」列给项目选一套后，此处才会出现按体系展开的可填结构。'
        }
      />
    );
  }

  // 字段按 doc_kind 分组：project 最先，其余按体系内定义顺序。
  const docKinds = [...new Set(model.fields.map((f) => f.doc_kind))].sort((a, b) =>
    a === 'project' ? -1 : b === 'project' ? 1 : 0,
  );

  return (
    <Space direction="vertical" className="w-full!" size={compact ? 8 : 12}>
      {error && <Alert type="error" showIcon message={error} />}

      <div className="flex flex-wrap items-center justify-between gap-2">
        <Typography.Text type="secondary" className="text-xs">
          项目级一份，全项目文档共享 —— 整存替换：改动先在内存，点「保存数据」一次性写回。
          {model.fields.length} 个字段 · {model.tables.length} 张表 · 已填 {countNonEmptyFields} 个字段。
        </Typography.Text>
        <Space>
          <Button size="small" onClick={() => void load()}>
            重新加载
          </Button>
          <Button size="small" type="primary" loading={saving} onClick={() => void save()}>
            保存数据
          </Button>
        </Space>
      </div>

      <Divider className="mb-1! mt-1!">
        <Typography.Text type="secondary" className="text-xs">
          标量字段（正文 {`{字段}`} 套话展开）
        </Typography.Text>
      </Divider>

      {docKinds.map((dk) => {
        const group = model.fields.filter((f) => f.doc_kind === dk);
        if (group.length === 0) return null;
        return (
          <div key={dk} className="mb-1!">
            <Tag className="mb-1!">{docKindLabel(dk)}</Tag>
            <div className="grid grid-cols-1 gap-1.5 md:grid-cols-2 xl:grid-cols-3">
              {group.map((f) => (
                <label key={f.id} className="block text-[13px]">
                  <Typography.Text strong className="mb-0.5! block">
                    {f.label || f.field_key}
                  </Typography.Text>
                  <Input
                    size="small"
                    value={f.value}
                    placeholder="（空）"
                    onChange={(e) => patchField(f.id, e.target.value)}
                  />
                </label>
              ))}
            </div>
          </div>
        );
      })}

      {model.tables.length === 0 && (
        <Typography.Text type="secondary" className="text-xs">
          该体系没有定义表结构。
        </Typography.Text>
      )}

      {model.tables.map((t) => (
        <TableEditor
          key={t.id}
          table={t}
          onCell={patchCell}
          onAdd={() => addRow(t.id)}
          onDel={(rid) => delRow(t.id, rid)}
          countHint={countTableRows(t)}
        />
      ))}
    </Space>
  );
}

// ---------------------------------------------------------------------------
// 单表编辑器（自定义列 → 每列一格输入）
// ---------------------------------------------------------------------------

function TableEditor({
  table,
  onCell,
  onAdd,
  onDel,
  countHint,
}: {
  table: MTable;
  onCell: (tableId: string, rid: string, colKey: string, value: string) => void;
  onAdd: () => void;
  onDel: (rid: string) => void;
  countHint: number;
}) {
  const columns: ColumnsType<MRow> = table.columns.map((c) => ({
    title: c.label || c.column_key,
    key: c.column_key,
    width: c.column_key === 'seq' ? 60 : 180,
    render: (_: unknown, r: MRow) => {
      // 序号列由行序给出，存值列如 seq 仍可手填（按需取舍）
      if (c.column_key === 'seq') {
        const v = r.cells['seq'] ?? '';
        return (
          <Input
            size="small"
            value={v}
            placeholder={`${table.rows.indexOf(r) + 1}`}
            onChange={(e) => onCell(table.id, r.rid, c.column_key, e.target.value)}
          />
        );
      }
      return (
        <Input
          size="small"
          value={r.cells[c.column_key] ?? ''}
          onChange={(e) => onCell(table.id, r.rid, c.column_key, e.target.value)}
        />
      );
    },
  }));
  columns.push({
    title: '',
    key: '__op',
    width: 44,
    render: (_, r) => (
      <Popconfirm title="删除该行？" okText="删除" cancelText="取消" onConfirm={() => onDel(r.rid)}>
        <Button size="small" type="text" danger aria-label="删除该行">
          删
        </Button>
      </Popconfirm>
    ),
  });

  return (
    <Card
      size="small"
      title={
        <span className="inline-flex items-center gap-2">
          {table.label || table.table_key}
          <Tag>{table.table_key}</Tag>
          {countHint > 0 && <Tag color="blue">{countHint} 行</Tag>}
        </span>
      }
    >
      <div className="overflow-x-auto">
        <Table<MRow>
          size="small"
          rowKey="rid"
          dataSource={table.rows}
          columns={columns}
          pagination={false}
          scroll={{ x: Math.max(560, table.columns.length * 160) }}
        />
      </div>
      <Button block type="dashed" size="small" className="mt-1!" onClick={onAdd}>
        ＋ 添加一行
      </Button>
    </Card>
  );
}
