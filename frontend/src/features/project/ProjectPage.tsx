// 项目管理页：项目的新增 / 重命名 / 删除 + 绑定「体系（执行方式）」+ 查看/修改项目数据。
// 项目是「大模块执行依据」：追踪数据（0007）与执行清单（0008）按项目隔离；体系（0009→0010
// 拆分后）定义「这个项目要哪些数据」的结构，数据值在此页全量可看可改（展开行编辑）。
// 文档模板仍全局通用、不分项目。

import { useEffect, useState } from 'react';
import {
  App,
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from 'antd';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type { Project } from '../../app/types';
import { errMsg, formatDateTime } from '../../app/format';
import ErrorAlert from '../../components/ErrorAlert';
import PageHeader from '../../components/PageHeader';
import ProjectDataEditor from '../projectdata/ProjectDataEditor';
import { listSchemes } from '../scheme/schemeSlice';
import {
  bindProjectScheme,
  createProject,
  deleteProject,
  loadProjects,
  renameProject,
  setCurrentProjectId,
} from './projectSlice';

interface CreateFormValues {
  name: string;
  scheme_id?: string | null;
}

export default function ProjectPage() {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();
  const { projects, currentProjectId, loading, error } = useAppSelector(
    (s) => s.projects,
  );
  const schemes = useAppSelector((s) => s.scheme.list);

  const [createOpen, setCreateOpen] = useState(false);
  const [renameId, setRenameId] = useState<string | null>(null);
  const [createForm] = Form.useForm<CreateFormValues>();
  const [renameForm] = Form.useForm<{ name: string }>();

  useEffect(() => {
    dispatch(loadProjects());
    dispatch(listSchemes());
  }, [dispatch]);

  const schemeName = (id: string | null) => schemes.find((s) => s.id === id)?.name;

  async function onCreate(v: CreateFormValues) {
    const name = v.name.trim();
    if (!name) return;
    try {
      const created = await dispatch(
        createProject({ name, scheme_id: v.scheme_id ?? null }),
      ).unwrap();
      dispatch(setCurrentProjectId(created.id)); // 建完直接切到新项目（空白面板）
      setCreateOpen(false);
      createForm.resetFields();
      message.success(`已创建项目「${name}」`);
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  function openRename(p: Project) {
    setRenameId(p.id);
    renameForm.setFieldsValue({ name: p.name });
  }

  async function onRename(v: { name: string }) {
    if (!renameId) return;
    const name = v.name.trim();
    if (!name) return;
    try {
      await dispatch(renameProject({ id: renameId, name })).unwrap();
      setRenameId(null);
      renameForm.resetFields();
      message.success('已重命名项目');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function onBindScheme(p: Project, scheme_id: string | null) {
    if (scheme_id === p.scheme_id) return; // 相同 = 幂等，不触发清数据
    try {
      await dispatch(bindProjectScheme({ id: p.id, scheme_id })).unwrap();
      if (scheme_id == null) {
        message.success('已解绑体系（该项目数据随之清空，待重新绑定后填写）');
      } else {
        const name = schemeName(scheme_id) ?? '';
        message.success(
          p.scheme_id
            ? `已切换为「${name}」——若原体系下已填过数据，已一并清空（防跨体系串写）。`
            : `已绑定体系「${name}」。展开该行即可按此体系填写项目数据。`,
        );
      }
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function onDelete(p: Project) {
    try {
      await dispatch(deleteProject(p.id)).unwrap();
      message.success(`已删除项目「${p.name}」及其全部数据`);
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  const schemeOptions = schemes.map((s) => ({ value: s.id, label: s.name }));

  return (
    <Space direction="vertical" className="w-full!" size={12}>
      <PageHeader
        title="项目管理"
        description="项目 = 执行依据：绑定一套「体系（执行方式）」决定这个项目要填哪些数据。删除项目会级联清空其下追踪 / 执行 / 项目数据。点行首「＋」展开可查看并修改该项目的全部数据（与执行清单共用同一份值）。"
        extra={
          <Button type="primary" onClick={() => setCreateOpen(true)}>
            ＋ 新增项目
          </Button>
        }
      />

      <ErrorAlert error={error} />

      <Table<Project>
        rowKey="id"
        dataSource={projects}
        loading={loading}
        pagination={{
          pageSize: 10,
          showSizeChanger: false,
          showTotal: (t) => `共 ${t} 条`,
        }}
        expandable={{
          expandedRowRender: (p) => (
            <div className="px-2! py-1!">
              <ProjectDataEditor
                key={`${p.id}-${p.scheme_id ?? 'none'}`}
                projectId={p.id}
              />
            </div>
          ),
          rowExpandable: () => true,
        }}
        columns={[
          {
            title: '项目名称',
            dataIndex: 'name',
            render: (name: string, p) => (
              <Space>
                <Typography.Text strong>{name}</Typography.Text>
                {p.id === currentProjectId && <Tag color="blue">当前项目</Tag>}
              </Space>
            ),
          },
          {
            title: '体系（执行方式）',
            dataIndex: 'scheme_id',
            width: 230,
            render: (sid: string | null, p) => (
              <Select
                className="w-full!"
                size="small"
                allowClear
                showSearch
                placeholder={schemes.length === 0 ? '（尚无体系，先到「台账编排」建）' : '未选择'}
                value={sid ?? undefined}
                options={schemeOptions}
                disabled={schemes.length === 0}
                onChange={(v) => void onBindScheme(p, v ?? null)}
              />
            ),
          },
          {
            title: 'ID',
            dataIndex: 'id',
            width: 260,
            render: (id: string) => (
              <Typography.Text code className="text-xs!" ellipsis={{ tooltip: id }}>
                {id}
              </Typography.Text>
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
            width: 180,
            render: (_, p) => (
              <Space>
                <Button size="small" onClick={() => openRename(p)}>
                  重命名
                </Button>
                <Popconfirm
                  title="删除该项目？"
                  description="会一并清空该项目下的追踪数据、执行清单与项目数据。"
                  okText="删除"
                  okButtonProps={{ danger: true }}
                  cancelText="取消"
                  onConfirm={() => onDelete(p)}
                >
                  <Button size="small" danger>
                    删除
                  </Button>
                </Popconfirm>
              </Space>
            ),
          },
        ]}
      />

      {/* 新增项目 */}
      <Modal
        title="新增项目"
        open={createOpen}
        onCancel={() => {
          setCreateOpen(false);
          createForm.resetFields();
        }}
        okText="创建"
        cancelText="取消"
        onOk={() => createForm.submit()}
        destroyOnClose
      >
        <Form form={createForm} layout="vertical" onFinish={onCreate}>
          <Form.Item
            label="项目名称"
            name="name"
            rules={[{ required: true, whitespace: true, message: '请填写项目名称' }]}
          >
            <Input placeholder="如：XX 型号测试" maxLength={60} />
          </Form.Item>
          <Form.Item label="体系（执行方式）" name="scheme_id">
            <Select
              allowClear
              showSearch
              placeholder="先不绑定（之后可在列表中选）"
              options={schemeOptions}
            />
          </Form.Item>
          <Typography.Paragraph type="secondary" className="mb-0! text-xs">
            体系决定这个项目要填哪些数据。新建项目得到一张空白追踪面板；选定体系后，展开该行或到
            「执行清单」按此体系填数据，装配时由模板引用。
          </Typography.Paragraph>
        </Form>
      </Modal>

      {/* 重命名 */}
      <Modal
        title="重命名项目"
        open={renameId !== null}
        onCancel={() => {
          setRenameId(null);
          renameForm.resetFields();
        }}
        okText="保存"
        cancelText="取消"
        onOk={() => renameForm.submit()}
        destroyOnClose
      >
        <Form form={renameForm} layout="vertical" onFinish={onRename}>
          <Form.Item
            label="项目名称"
            name="name"
            rules={[{ required: true, whitespace: true, message: '请填写项目名称' }]}
          >
            <Input maxLength={60} />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  );
}
