// 测试项编写页：为需求创建测试项。
// 每个新建的测试项 = 追踪图里的一个 `test_item` 节点，并直接 `derives_from` 到
// 所选需求（可关联多个）。文档模板是结构的抽象、测试项是具体事件——这里建测试项
// **不**改动文档模板，模板里「测试项章节」的动态展示由后续功能联动。
//
// 数据面复用 traceSlice（loadTraceGraph / clearGraph），与本项目追踪图同一数据源：
// 本页展示的节点 = 需求 + 测试项，二者在追踪图里就是同一批数据。

import { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  App,
  Button,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from 'antd';
import { api } from '../../app/api';
import { errMsg, formatDateTime } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type { TraceLink, TraceNode } from '../../app/types';
import ErrorAlert from '../../components/ErrorAlert';
import NoProjectEmpty from '../../components/NoProjectEmpty';
import PageHeader from '../../components/PageHeader';
import { clearGraph, loadTraceGraph } from '../trace/traceSlice';

interface ItemRow extends TraceNode {
  reqs: TraceNode[];
}

export default function TestItemPage() {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();
  const projectId = useAppSelector((s) => s.projects.currentProjectId);
  const { nodes, links, loading, error } = useAppSelector((s) => s.trace);

  const [createOpen, setCreateOpen] = useState(false);
  const [form] = Form.useForm<{
    title: string;
    module?: string;
    description?: string;
    requirement_ids: string[];
  }>();

  useEffect(() => {
    dispatch(clearGraph()); // 切项目先清上一项目节点
    if (projectId) dispatch(loadTraceGraph(projectId));
  }, [dispatch, projectId]);

  // 需求 / 测试项 / 测试项→需求 的映射都从同一项目图上派生
  const requirements = useMemo(() => {
    return nodes
      .filter((n) => n.kind === 'software_requirement')
      .sort((a, b) => a.sort_key - b.sort_key || a.created_at.localeCompare(b.created_at));
  }, [nodes]);

  const rows: ItemRow[] = useMemo(() => {
    const reqById = new Map(requirements.map((r) => [r.id, r]));
    const reqOfItem = new Map<string, TraceNode[]>();
    for (const l of links) {
      if (l.link_type !== 'derives_from') continue;
      const req = reqById.get(l.target_node_id);
      if (!req) continue;
      const key = l.source_node_id;
      reqOfItem.set(key, [...(reqOfItem.get(key) ?? []), req]);
    }
    return nodes
      .filter((n) => n.kind === 'test_item')
      .map((n) => ({ ...n, reqs: reqOfItem.get(n.id) ?? [] }))
      .sort((a, b) => a.created_at.localeCompare(b.created_at) || a.id.localeCompare(b.id));
  }, [nodes, links, requirements]);

  async function onCreate(v: {
    title: string;
    module?: string;
    description?: string;
    requirement_ids: string[];
  }) {
    if (!projectId) return;
    try {
      await api.post<TraceNode>(`/trace/test-items?project_id=${projectId}`, {
        title: v.title.trim(),
        module: v.module?.trim() || null,
        description: v.description?.trim() || null,
        requirement_ids: v.requirement_ids,
      });
      setCreateOpen(false);
      form.resetFields();
      message.success('测试项已创建，并已关联所选需求（追踪图会同步出现该节点）');
      dispatch(loadTraceGraph(projectId));
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  if (!projectId) {
    return (
      <NoProjectEmpty description="测试项属于具体项目，请先在右上角选择一个项目（或在「项目管理」页新建）。" />
    );
  }

  return (
    <Space direction="vertical" className="w-full!" size={12}>
      <PageHeader
        title="测试项编写"
        description="为需求设计测试项：创建时必选 ≥1 个需求，建好的测试项会作为节点出现在「需求追踪」图的测试项泳道并派生自已选需求（可关联多个）；不会改动文档模板。"
        extra={
          <Button
            type="primary"
            disabled={requirements.length === 0}
            onClick={() => setCreateOpen(true)}
          >
            ＋ 新增测试项
          </Button>
        }
      />

      <ErrorAlert error={error} />
      {requirements.length === 0 && !loading && (
        <Alert
          type="info"
          showIcon
          message="当前项目还没有需求节点"
          description="先在「需求追踪」页为该项目新增需求，才能为它编写测试项。"
        />
      )}

      <Table<ItemRow>
        rowKey="id"
        dataSource={rows}
        loading={loading}
        pagination={{
          pageSize: 10,
          showSizeChanger: false,
          showTotal: (t) => `共 ${t} 个测试项`,
        }}
        columns={[
          {
            title: '标题',
            render: (_, it) => (
              <Space direction="vertical" size={0}>
                <Typography.Text strong>{it.title}</Typography.Text>
                <Typography.Text type="secondary" className="text-[11px]">
                  {it.external_ref}
                </Typography.Text>
              </Space>
            ),
          },
          {
            title: '模块',
            dataIndex: 'module',
            width: 140,
            render: (v: string | null) => v ?? '-',
          },
          {
            title: '关联需求（derives_from）',
            width: 340,
            render: (_, it) =>
              it.reqs.length === 0 ? (
                <Typography.Text type="secondary">（无）</Typography.Text>
              ) : (
                <Space wrap size={4}>
                  {it.reqs.map((r) => (
                    <Tag key={r.id} color="geekblue">
                      {r.external_ref} · {r.title}
                    </Tag>
                  ))}
                </Space>
              ),
          },
          {
            title: '创建时间',
            dataIndex: 'created_at',
            width: 180,
            render: (v: string) => formatDateTime(v),
          },
        ]}
      />

      {/* 新增测试项 */}
      <Modal
        title="新增测试项"
        open={createOpen}
        onCancel={() => {
          setCreateOpen(false);
          form.resetFields();
        }}
        okText="创建"
        cancelText="取消"
        onOk={() => form.submit()}
        destroyOnClose
      >
        <Form form={form} layout="vertical" onFinish={onCreate}>
          <Form.Item
            label="测试项标题"
            name="title"
            rules={[{ required: true, whitespace: true, message: '请填写测试项标题' }]}
          >
            <Input placeholder="如：账号密码登录校验" maxLength={120} />
          </Form.Item>
          <Form.Item label="模块（软分组，可空）" name="module">
            <Input placeholder="如：登录" maxLength={40} />
          </Form.Item>
          <Form.Item label="描述（可空）" name="description">
            <Input.TextArea rows={3} placeholder="该测试项要验证什么 / 关注点…" />
          </Form.Item>
          <Form.Item
            label="关联需求（可多选）"
            name="requirement_ids"
            rules={[
              { required: true, message: '至少选择一个需求' },
              {
                validator: (_, v: string[] | undefined) =>
                  !v || v.length === 0
                    ? Promise.reject(new Error('至少选择一个需求'))
                    : Promise.resolve(),
              },
            ]}
          >
            <Select
              mode="multiple"
              placeholder="选择该测试项覆盖的需求…"
              maxTagCount={4}
              options={requirements.map((r) => ({
                value: r.id,
                label: `${r.external_ref} · ${r.title}`,
              }))}
            />
          </Form.Item>
          <Typography.Paragraph type="secondary" className="mb-0! text-xs">
            创建后该测试项直接派生自已选需求（一条 derives_from 边 / 每个需求），追踪页测试项泳道会同步新增该节点。
          </Typography.Paragraph>
        </Form>
      </Modal>
    </Space>
  );
}
