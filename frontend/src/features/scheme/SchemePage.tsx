// 台账编排（体系设计）页：设计「这类项目需要什么数据」——可复用、可多套的体系。
//
// 体系 = 标量字段（doc_kind 作用域 + 字段名 + 标签）+ 表（表键 + 标签 + 自定义列）。
// 与旧「项目台账」的分界：这里只定义 **结构**（schema），不再填数据；项目在「项目管理」
// 页按执行方式绑定一套，值在「执行清单 / 项目管理」的数据区填写（ProjectDataEditor）。
// 整存替换（PUT /schemes/{id}）：服务端按自然键 upsert 保 id → 已填项目值不丢；只有
// 真正删掉的字段/表才级联清掉对应项目值。
//
// 该页全局通用、不按项目收口。

import { useEffect, useState } from 'react';
import {
  App,
  Button,
  Card,
  Empty,
  Input,
  List,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tag,
  Typography,
} from 'antd';
import { api } from '../../app/api';
import { errMsg } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type { SchemeFull, SchemeSpec } from '../../app/types';
import { DOC_KIND_OPTIONS } from '../../app/types';
import ErrorAlert from '../../components/ErrorAlert';
import PageHeader from '../../components/PageHeader';
import {
  createScheme,
  deleteScheme,
  listSchemes,
  loadScheme,
  saveScheme,
} from './schemeSlice';

// ---------------------------------------------------------------------------
// 编辑期草稿（无 id/sort_key；保存时组 SchemeSpec，服务端 upsert 保 id）
// ---------------------------------------------------------------------------

interface FDraft {
  rid: string;
  doc_kind: string;
  field_key: string;
  label: string;
}

interface CDraft {
  rid: string;
  column_key: string;
  label: string;
}

interface TDraft {
  rid: string;
  table_key: string;
  label: string;
  columns: CDraft[];
}

interface SDraft {
  name: string;
  description: string;
  fields: FDraft[];
  tables: TDraft[];
}

function uid(): string {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `r-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function fromFull(full: SchemeFull): SDraft {
  return {
    name: full.scheme.name,
    description: full.scheme.description,
    fields: full.fields.map((f) => ({
      rid: f.id,
      doc_kind: f.doc_kind,
      field_key: f.field_key,
      label: f.label,
    })),
    tables: full.tables.map((t) => ({
      rid: t.table.id,
      table_key: t.table.table_key,
      label: t.table.label,
      columns: t.columns.map((c) => ({
        rid: c.id,
        column_key: c.column_key,
        label: c.label,
      })),
    })),
  };
}

/** 草稿 → SchemeSpec（sort_key / label 兜底 / 去重校验由服务端处理）。 */
function toSpec(d: SDraft): SchemeSpec {
  return {
    name: d.name,
    description: d.description,
    fields: d.fields.map((f) => ({
      doc_kind: f.doc_kind || 'project',
      field_key: f.field_key,
      label: f.label,
    })),
    tables: d.tables.map((t) => ({
      table_key: t.table_key,
      label: t.label,
      columns: t.columns.map((c) => ({ column_key: c.column_key, label: c.label })),
    })),
  };
}

/** full（含 id）→ 复制用的 SchemeSpec（字段/表自然键保留 → 新体系里复用相同键）。 */
function specFromFull(full: SchemeFull): SchemeSpec {
  return {
    name: full.scheme.name,
    description: full.scheme.description,
    fields: full.fields.map((f) => ({
      doc_kind: f.doc_kind,
      field_key: f.field_key,
      label: f.label,
    })),
    tables: full.tables.map((t) => ({
      table_key: t.table.table_key,
      label: t.table.label,
      columns: t.columns.map((c) => ({ column_key: c.column_key, label: c.label })),
    })),
  };
}

const DOC_KIND_OPTIONS_LIST = DOC_KIND_OPTIONS.map((v) => ({
  value: v,
  label: v === 'project' ? 'project（项目共享）' : v,
}));

// ---------------------------------------------------------------------------
// 小控件：一行字段 / 一行列
// ---------------------------------------------------------------------------

function FieldRow({
  value,
  onChange,
  onDelete,
}: {
  value: FDraft;
  onChange: (next: FDraft) => void;
  onDelete: () => void;
}) {
  return (
    <div className="grid grid-cols-[150px_1fr_1fr_34px] items-center gap-1.5">
      <Select
        size="small"
        value={value.doc_kind || 'project'}
        onChange={(doc_kind: string) => onChange({ ...value, doc_kind })}
        options={DOC_KIND_OPTIONS_LIST}
      />
      <Input
        size="small"
        value={value.field_key}
        placeholder="字段名（= {占位}）"
        onChange={(e) => onChange({ ...value, field_key: e.target.value })}
      />
      <Input
        size="small"
        value={value.label}
        placeholder="标签（缺省 = 字段名）"
        onChange={(e) => onChange({ ...value, label: e.target.value })}
      />
      <Button size="small" type="text" danger onClick={onDelete} aria-label="删除字段">
        删
      </Button>
    </div>
  );
}

function ColumnRow({
  value,
  onChange,
  onDelete,
}: {
  value: CDraft;
  onChange: (next: CDraft) => void;
  onDelete: () => void;
}) {
  return (
    <div className="grid grid-cols-[1fr_1fr_34px] items-center gap-1.5">
      <Input
        size="small"
        value={value.column_key}
        placeholder="列键（如 name / party_kind / seq）"
        onChange={(e) => onChange({ ...value, column_key: e.target.value })}
      />
      <Input
        size="small"
        value={value.label}
        placeholder="列标题（缺省 = 列键）"
        onChange={(e) => onChange({ ...value, label: e.target.value })}
      />
      <Button size="small" type="text" danger onClick={onDelete} aria-label="删除列">
        删
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 页面
// ---------------------------------------------------------------------------

export default function SchemePage() {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();
  const { list, current, loading, saving, error } = useAppSelector((s) => s.scheme);

  const [draft, setDraft] = useState<SDraft | null>(null);

  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [creating, setCreating] = useState(false);

  const [copyOpen, setCopyOpen] = useState(false);
  const [copyName, setCopyName] = useState('');
  const [copySource, setCopySource] = useState<SchemeFull | null>(null);

  useEffect(() => {
    dispatch(listSchemes());
  }, [dispatch]);

  // 当前定义（slice）变化 → 重建编辑草稿
  useEffect(() => {
    setDraft(current ? fromFull(current) : null);
  }, [current]);

  async function onCreate() {
    const name = createName.trim();
    if (!name) {
      message.warning('请填写体系名称');
      return;
    }
    setCreating(true);
    try {
      await dispatch(createScheme({ name, description: '' })).unwrap();
      setCreateName('');
      message.success(`已建体系「${name}」，开始设计它的字段与表结构`);
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setCreating(false);
    }
  }

  async function onSave() {
    if (!draft || !current) return;
    try {
      await dispatch(saveScheme({ id: current.scheme.id, spec: toSpec(draft) })).unwrap();
      message.success('体系已保存（自然键 upsert 保 id：已填项目值不丢）');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function openCopyFrom(id: string) {
    try {
      const full =
        current?.scheme.id === id ? current : await api.get<SchemeFull>(`/schemes/${id}`);
      setCopySource(full);
      setCopyName(`${full.scheme.name}（副本）`);
      setCopyOpen(true);
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function onCopy() {
    if (!copySource) return;
    const name = copyName.trim();
    if (!name) {
      message.warning('请填写副本名称');
      return;
    }
    setCreating(true);
    try {
      const spec = specFromFull(copySource);
      spec.name = name;
      await dispatch(createScheme(spec)).unwrap();
      setCopyOpen(false);
      setCopySource(null);
      message.success(`已另存为「${name}」`);
    } catch (e) {
      message.error(errMsg(e));
    } finally {
      setCreating(false);
    }
  }

  async function onDelete(id: string) {
    try {
      await dispatch(deleteScheme(id)).unwrap();
      message.success('体系已删除');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  const patchDraft = (p: Partial<SDraft>) => setDraft((d) => (d ? { ...d, ...p } : d));

  function patchField(rid: string, next: FDraft) {
    patchDraft({ fields: (draft?.fields ?? []).map((f) => (f.rid === rid ? next : f)) });
  }
  const addField = () =>
    patchDraft({
      fields: [...(draft?.fields ?? []), { rid: uid(), doc_kind: 'project', field_key: '', label: '' }],
    });
  const delField = (rid: string) =>
    patchDraft({ fields: (draft?.fields ?? []).filter((f) => f.rid !== rid) });

  function patchTable(rid: string, next: TDraft) {
    patchDraft({ tables: (draft?.tables ?? []).map((t) => (t.rid === rid ? next : t)) });
  }
  const delTable = (rid: string) =>
    patchDraft({ tables: (draft?.tables ?? []).filter((t) => t.rid !== rid) });
  const addTable = () =>
    patchDraft({
      tables: [...(draft?.tables ?? []), { rid: uid(), table_key: '', label: '', columns: [] }],
    });

  function patchColumn(trid: string, crid: string, next: CDraft) {
    const t = (draft?.tables ?? []).find((x) => x.rid === trid);
    if (t) {
      patchTable(trid, { ...t, columns: t.columns.map((c) => (c.rid === crid ? next : c)) });
    }
  }
  function addColumn(trid: string) {
    const t = (draft?.tables ?? []).find((x) => x.rid === trid);
    if (t) patchTable(trid, { ...t, columns: [...t.columns, { rid: uid(), column_key: '', label: '' }] });
  }
  function delColumn(trid: string, crid: string) {
    const t = (draft?.tables ?? []).find((x) => x.rid === trid);
    if (t) patchTable(trid, { ...t, columns: t.columns.filter((c) => c.rid !== crid) });
  }

  const filledFields = draft?.fields.filter((f) => f.field_key.trim()).length ?? 0;
  const tableCount = draft?.tables.length ?? 0;

  return (
    <Space direction="vertical" className="w-full!" size={12}>
      <PageHeader
        title="台账编排（体系设计）"
        description="设计「这类项目要哪些数据」——可复用、可多套的体系（= 项目执行方式）。字段/表只定义结构；值在执行清单与项目管理的数据区填。模板按字段/表键引用，装配引擎取「体系结构 × 已填值」出正文。"
        extra={
          <Button type="primary" onClick={() => setCreateOpen(true)}>
            ＋ 新建体系
          </Button>
        }
      />
      <ErrorAlert error={error} />

      <div className="flex items-start gap-3">
        {/* 左：体系清单 */}
        <Card
          className="w-[320px]! flex-none"
          size="small"
          title={
            <span className="text-sm">
              体系清单<Tag className="ml-1!">{list.length}</Tag>
            </span>
          }
        >
          {list.length === 0 && loading ? (
            <div className="flex justify-center py-8">
              <Spin />
            </div>
          ) : list.length === 0 ? (
            <Empty description="还没有体系，先「新建体系」。" />
          ) : (
            <List
              size="small"
              dataSource={list}
              renderItem={(s) => {
                const active = s.id === current?.scheme.id;
                return (
                  <List.Item
                    className={`cursor-pointer rounded px-2! ${
                      active ? 'bg-blue-50!' : 'hover:bg-gray-50!'
                    }`}
                    onClick={() => dispatch(loadScheme(s.id))}
                    actions={[
                      <Button
                        key="copy"
                        type="text"
                        size="small"
                        onClick={(e) => {
                          e.stopPropagation();
                          void openCopyFrom(s.id);
                        }}
                      >
                        复制
                      </Button>,
                      <Popconfirm
                        key="del"
                        title="删除这套体系？"
                        description="已被项目绑定时会失败（先在项目管理页解绑）。"
                        okText="删除"
                        okButtonProps={{ danger: true }}
                        cancelText="取消"
                        onConfirm={() => void onDelete(s.id)}
                        onCancel={(e) => e?.stopPropagation()}
                      >
                        <Button
                          type="text"
                          size="small"
                          danger
                          onClick={(e) => e.stopPropagation()}
                        >
                          删除
                        </Button>
                      </Popconfirm>,
                    ]}
                  >
                    <div className="min-w-0 flex-1">
                      <Typography.Text strong className="block truncate">
                        {s.name}
                      </Typography.Text>
                      {s.description && (
                        <Typography.Text type="secondary" className="block truncate text-xs">
                          {s.description}
                        </Typography.Text>
                      )}
                    </div>
                  </List.Item>
                );
              }}
            />
          )}
        </Card>

        {/* 右：设计器 */}
        <Card className="min-w-0 flex-1" size="small" title="体系结构设计">
          {!draft ? (
            <div className="flex h-[300px] items-center justify-center">
              <Empty description="从左侧选一套体系开始编辑；「复制 / 另存为副本」可基于现有体系派生新执行方式。" />
            </div>
          ) : (
            <Space direction="vertical" className="w-full!" size={10}>
              <div className="flex items-center justify-between gap-2">
                <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
                  <Input
                    className="w-[200px]!"
                    value={draft.name}
                    onChange={(e) => patchDraft({ name: e.target.value })}
                    placeholder="体系名称"
                  />
                  <Input
                    className="w-[300px]!"
                    value={draft.description}
                    onChange={(e) => patchDraft({ description: e.target.value })}
                    placeholder="说明（可选）"
                  />
                  <Typography.Text type="secondary" className="text-xs">
                    {filledFields} 字段 · {tableCount} 表
                  </Typography.Text>
                </div>
                <Space>
                  <Button size="small" onClick={() => current && void openCopyFrom(current.scheme.id)}>
                    另存为副本
                  </Button>
                  <Button size="small" type="primary" loading={saving} onClick={() => void onSave()}>
                    保存体系
                  </Button>
                </Space>
              </div>

              <Typography.Text type="secondary" className="block text-xs leading-5">
                字段 doc_kind：project = 项目共享（任何文档都能展开）；文档类（大纲/说明/…）= 仅装配该类
                文档时优先命中。表的自定义列决定装配/填写的列头。删除已填过值的字段/表会级联清掉各项目
                对应数据——改结构请优先「保留键改名/改标签」。
              </Typography.Text>

              <Card
                size="small"
                title={<span className="text-sm">标量字段</span>}
                extra={<Tag>{filledFields}</Tag>}
              >
                {draft.fields.length === 0 && (
                  <Typography.Text type="secondary" className="mb-1! block text-xs">
                    还没有字段。字段名 = 模板里写的 {`{占位}`}（如 项目代号 / 文档标识）。
                  </Typography.Text>
                )}
                <Space direction="vertical" size={6} className="w-full!">
                  {draft.fields.map((f) => (
                    <FieldRow
                      key={f.rid}
                      value={f}
                      onChange={(next) => patchField(f.rid, next)}
                      onDelete={() => delField(f.rid)}
                    />
                  ))}
                  <Button block type="dashed" size="small" onClick={addField}>
                    ＋ 添加字段
                  </Button>
                </Space>
              </Card>

              <Space direction="vertical" className="w-full!" size={8}>
                {draft.tables.map((t) => (
                  <Card
                    key={t.rid}
                    size="small"
                    title={<span className="text-sm">表</span>}
                    extra={
                      <Button size="small" type="text" danger onClick={() => delTable(t.rid)}>
                        删除此表
                      </Button>
                    }
                  >
                    <Space direction="vertical" className="w-full!" size={6}>
                      <div className="grid grid-cols-[180px_1fr] items-center gap-1.5">
                        <Input
                          size="small"
                          value={t.table_key}
                          onChange={(e) => patchTable(t.rid, { ...t, table_key: e.target.value })}
                          placeholder="表键（模板 source 引用，如 parties）"
                        />
                        <Input
                          size="small"
                          value={t.label}
                          onChange={(e) => patchTable(t.rid, { ...t, label: e.target.value })}
                          placeholder="表标题（缺省 = 表键）"
                        />
                      </div>
                      <Typography.Text type="secondary" className="block text-xs">
                        自定义列（装配/填写都按这里的列头渲染）：
                      </Typography.Text>
                      {t.columns.length === 0 && (
                        <Typography.Text type="secondary" className="block text-xs">
                          暂无列。至少加一列才能在该表填写行。
                        </Typography.Text>
                      )}
                      <Space direction="vertical" size={6} className="w-full!">
                        {t.columns.map((c) => (
                          <ColumnRow
                            key={c.rid}
                            value={c}
                            onChange={(next) => patchColumn(t.rid, c.rid, next)}
                            onDelete={() => delColumn(t.rid, c.rid)}
                          />
                        ))}
                        <Button block type="dashed" size="small" onClick={() => addColumn(t.rid)}>
                          ＋ 添加列
                        </Button>
                      </Space>
                    </Space>
                  </Card>
                ))}
                <Button block type="dashed" onClick={addTable}>
                  ＋ 添加表
                </Button>
              </Space>
            </Space>
          )}
        </Card>
      </div>

      {/* 新建体系弹窗 */}
      <Modal
        title="新建体系（执行方式）"
        open={createOpen}
        okText="创建"
        cancelText="取消"
        confirmLoading={creating}
        onOk={() => void onCreate()}
        onCancel={() => {
          setCreateOpen(false);
          setCreateName('');
        }}
        destroyOnClose
      >
        <Space direction="vertical" className="w-full!" size={6}>
          <Input
            value={createName}
            onChange={(e) => setCreateName(e.target.value)}
            placeholder="体系名称，如：大纲测评 / 单元测试 / 第三方测评…"
            onPressEnter={() => void onCreate()}
          />
          <Typography.Text type="secondary" className="text-xs">
            新建后进入设计：添加字段与表（自定义列）。值不在这里填——等项目管理页把项目绑到这套体系后，
            在执行清单 / 项目管理的数据区填写。
          </Typography.Text>
        </Space>
      </Modal>

      {/* 另存为副本弹窗 */}
      <Modal
        title="另存为副本"
        open={copyOpen}
        okText="创建副本"
        cancelText="取消"
        confirmLoading={creating}
        onOk={() => void onCopy()}
        onCancel={() => {
          setCopyOpen(false);
          setCopySource(null);
        }}
        destroyOnClose
      >
        <Input
          value={copyName}
          onChange={(e) => setCopyName(e.target.value)}
          placeholder="副本名称"
          onPressEnter={() => void onCopy()}
        />
      </Modal>
    </Space>
  );
}
