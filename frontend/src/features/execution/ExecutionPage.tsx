import { useCallback, useEffect, useState } from 'react';
import {
  App,
  Button,
  Card,
  Drawer,
  Empty,
  Input,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from 'antd';
import { api } from '../../app/api';
import { errMsg, formatDateTime } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type {
  ExecutionDetail,
  ExecutionInstance,
  ExecutionItem,
  InstanceStatus,
  ItemStatus,
} from '../../app/types';
import {
  INSTANCE_STATUS_LABEL,
  ITEM_STATUS_LABEL,
} from '../../app/types';
import ProjectDataEditor from '../projectdata/ProjectDataEditor';
import ErrorAlert from '../../components/ErrorAlert';
import NoProjectEmpty from '../../components/NoProjectEmpty';
import PageHeader from '../../components/PageHeader';
import {
  createExecution,
  loadExecSetup,
  loadExecutionDetail,
  resetForProject,
  updateExecutionItem,
} from './executionSlice';

const INSTANCE_STATUS_COLOR: Record<InstanceStatus, string> = {
  draft: 'default',
  in_progress: 'processing',
  completed: 'success',
};

const ITEM_STATUS_COLOR: Record<ItemStatus, string> = {
  pending: 'default',
  in_progress: 'processing',
  completed: 'success',
  skipped: 'warning',
};

const ITEM_OPTIONS = (Object.keys(ITEM_STATUS_LABEL) as ItemStatus[]).map((s) => ({
  value: s,
  label: ITEM_STATUS_LABEL[s],
}));
const INSTANCE_OPTIONS = (Object.keys(INSTANCE_STATUS_LABEL) as InstanceStatus[]).map(
  (s) => ({ value: s, label: INSTANCE_STATUS_LABEL[s] }),
);

function textOf(item: ExecutionItem): string {
  const v = item.content?.['text'];
  return typeof v === 'string' ? v : '';
}

export default function ExecutionPage() {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();
  const { templates, instances, detail, loading, error } = useAppSelector(
    (s) => s.execution,
  );
  // 执行清单按项目隔离：数据面收口到当前选中项目（模板仍全局）
  const projectId = useAppSelector((s) => s.projects.currentProjectId);

  const [stageInput, setStageInput] = useState('');
  const [tplId, setTplId] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [activeStatus, setActiveStatus] = useState<InstanceStatus>('draft');

  useEffect(() => {
    if (!projectId) return;
    dispatch(resetForProject()); // 切项目先清上一项目实例，避免旧数据闪现
    dispatch(loadExecSetup(projectId));
  }, [dispatch, projectId]);

  function openDetail(id: string) {
    setDrawerOpen(true);
    dispatch(loadExecutionDetail(id));
  }

  const items = detail ? [...detail.items].sort((a, b) => a.sort_key - b.sort_key) : [];

  async function runCreate() {
    if (!projectId) return;
    if (!tplId) {
      message.warning('请选择模板');
      return;
    }
    try {
      const inst = await dispatch(
        createExecution({
          project_id: projectId,
          template_id: tplId,
          stage: stageInput.trim() || null,
        }),
      ).unwrap();
      await dispatch(loadExecSetup(projectId));
      setTplId(null);
      setStageInput('');
      openDetail(inst.id);
      message.success('执行清单已生成（由模板节点按文档序派生）');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  const commitItem = useCallback(
    async (id: string, patch: { status?: ItemStatus; content?: Record<string, unknown> }) => {
      try {
        await dispatch(updateExecutionItem({ id, ...patch })).unwrap();
        if (detail) dispatch(loadExecutionDetail(detail.instance.id));
      } catch (e) {
        message.error(errMsg(e));
      }
    },
    [dispatch, detail, message],
  );

  async function commitInstanceStatus(status: InstanceStatus) {
    if (!detail) return;
    setActiveStatus(status);
    try {
      await api.put(`/executions/instances/${detail.instance.id}`, { status });
      if (detail) dispatch(loadExecutionDetail(detail.instance.id));
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  useEffect(() => {
    if (detail) setActiveStatus(detail.instance.status);
  }, [detail]);

  const nameOfTemplate = (tid: string) =>
    templates.find((t) => t.id === tid)?.name ?? tid.slice(0, 8);

  if (!projectId) {
    return (
      <NoProjectEmpty description="执行清单按项目生成，请先在右上角选择一个项目（或在「项目管理」页新建）。" />
    );
  }

  return (
    <Space direction="vertical" className="w-full!" size={12}>
      <PageHeader
        title="执行清单"
        description="为当前项目选一份（全局）模板 + 执行阶段 → 按模板文档序（大章节竖向、同级子章节横向，竖向最上 = 文档最前）摊开待办清单，落在此项目下；执行人员逐项填写内容（内容落库，结构不落库）。"
      />
      <ErrorAlert error={error} />

      <Space wrap>
        <Select
          className="w-[280px]!"
          placeholder="选择模板"
          value={tplId}
          onChange={setTplId}
          options={templates.map((t) => ({
            value: t.id,
            label: `${t.name}（${t.kind}）`,
          }))}
        />
        <Input
          className="w-[200px]!"
          placeholder="执行阶段，如：第一轮测试"
          value={stageInput}
          onChange={(e) => setStageInput(e.target.value)}
          onPressEnter={runCreate}
        />
        <Button type="primary" onClick={runCreate} loading={loading}>
          生成执行实例
        </Button>
        <Button onClick={() => projectId && dispatch(loadExecSetup(projectId))}>
          刷新
        </Button>
      </Space>

      <Table<ExecutionInstance>
        size="small"
        rowKey="id"
        dataSource={instances}
        loading={loading}
        pagination={{ pageSize: 8 }}
        columns={[
          {
            title: '模板',
            dataIndex: 'template_id',
            width: 220,
            render: (tid: string) => nameOfTemplate(tid),
          },
          { title: '执行阶段', dataIndex: 'stage', render: (v: string | null) => v ?? '-' },
          {
            title: '状态',
            dataIndex: 'status',
            width: 120,
            render: (s: InstanceStatus) => (
              <Tag color={INSTANCE_STATUS_COLOR[s]}>{INSTANCE_STATUS_LABEL[s]}</Tag>
            ),
          },
          {
            title: '创建时间',
            dataIndex: 'created_at',
            width: 200,
            render: (v: string) => formatDateTime(v),
          },
          {
            title: '操作',
            key: 'op',
            width: 100,
            render: (_, inst) => (
              <Button size="small" onClick={() => openDetail(inst.id)}>
                查看清单
              </Button>
            ),
          },
        ]}
      />

      <Drawer
        title="执行清单"
        width={860}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
      >
        {!detail ? (
          <Empty description="选择一条实例查看清单" />
        ) : (
          <Space direction="vertical" className="w-full!" size={12}>
            <Space wrap>
              <Typography.Text strong>{nameOfTemplate(detail.instance.template_id)}</Typography.Text>
              <Tag>{detail.instance.stage ?? '未分阶段'}</Tag>
              <Select
                className="w-[130px]!"
                value={activeStatus}
                onChange={(s: InstanceStatus) => commitInstanceStatus(s)}
                options={INSTANCE_OPTIONS}
              />
            </Space>

            {/* 执行清单 = 唯一填数面：在此填的字段/表落「项目级一份」，装配据此出正文 */}
            <Card
              size="small"
              title={
                <span className="text-[13px]">
                  项目数据
                  <Typography.Text type="secondary" className="ml-1! text-xs">
                    （按本实例所属项目的体系展开；保存即写回，与「项目管理」页数据区同一份）
                  </Typography.Text>
                </span>
              }
            >
              <ProjectDataEditor
                key={detail.instance.project_id}
                projectId={detail.instance.project_id}
                compact
              />
            </Card>

            <Typography.Text type="secondary" className="text-xs">
              {detail.items.length} 项 · 节点内容编辑后自动保存（人工槽：填内容 → 装配时落到该节点）
            </Typography.Text>
            {items.map((item, idx) => (
              <Space key={item.id} direction="vertical" size={4} className="w-full!">
                <Space wrap>
                  <Tag>{idx + 1}</Tag>
                  <Typography.Text>{item.title}</Typography.Text>
                  <Select
                    className="w-[120px]!"
                    size="small"
                    value={item.status}
                    onChange={(s: ItemStatus) => commitItem(item.id, { status: s })}
                    options={ITEM_OPTIONS}
                  />
                </Space>
                <Input.TextArea
                  rows={2}
                  defaultValue={textOf(item)}
                  placeholder="执行人员在此填写该节点产出的内容…"
                  onBlur={(e) => {
                    const text = e.target.value;
                    if (text !== textOf(item)) {
                      commitItem(item.id, { content: { text } });
                    }
                  }}
                />
              </Space>
            ))}
          </Space>
        )}
      </Drawer>
    </Space>
  );
}
