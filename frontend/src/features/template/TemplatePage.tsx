import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  App,
  Button,
  Divider,
  Empty,
  Form,
  Input,
  Layout,
  Modal,
  Select,
  Space,
  Tabs,
  Tag,
  Tree,
  TreeSelect,
  Typography,
} from 'antd';
import type { TreeDataNode, TreeProps } from 'antd';
import { errMsg } from '../../app/format';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import type {
  DocumentTemplate,
  TemplateNodeType,
} from '../../app/types';
import { NODE_TYPE_LABEL } from '../../app/types';
import ErrorAlert from '../../components/ErrorAlert';
import AssemblePreview from './AssemblePreview';
import ContentComposer from './ContentComposer';
import TemplateCanvas from './TemplateCanvas';
import { NODE_TYPE_COLOR, childrenMap, containsParentMap } from './layout';
import ContextMenu, { clampMenuPos } from '../../components/ContextMenu';
import {
  createEdge,
  createNode,
  createTemplate,
  deleteEdge,
  deleteNode,
  loadTemplateFull,
  loadTemplates,
  reorderNodes,
  updateNode,
} from './templateSlice';

const { Sider, Content } = Layout;

const KIND_OPTIONS = ['大纲', '报告'];
const TYPE_OPTIONS = (Object.keys(NODE_TYPE_LABEL) as TemplateNodeType[]).map((t) => ({
  value: t,
  label: NODE_TYPE_LABEL[t],
}));

type TreeDropInfo = Parameters<NonNullable<TreeProps['onDrop']>>[0];
type AddMode = 'root' | 'child' | 'sibling';

interface MoveTreeNode {
  value: string;
  title: string;
  children?: MoveTreeNode[];
}

const ADD_MODE_TITLE: Record<AddMode, string> = {
  root: '新增一级节点',
  child: '新增子节点',
  sibling: '新增同级节点',
};

export default function TemplatePage() {
  const dispatch = useAppDispatch();
  const { message, modal } = App.useApp();
  const { templates, current, loading, error } = useAppSelector((s) => s.template);

  const [activeId, setActiveId] = useState<string | null>(null);
  const [selNodeId, setSelNodeId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createForm] = Form.useForm<{ name: string; kind: string }>();

  // 新增节点弹窗
  const [addState, setAddState] = useState<{ mode: AddMode; refId: string | null } | null>(
    null,
  );
  const [addForm] = Form.useForm<{ node_type: TemplateNodeType; title: string }>();

  // 重命名弹窗
  const [renameId, setRenameId] = useState<string | null>(null);
  const [renameTitle, setRenameTitle] = useState('');

  // 「移到…下面」弹窗：把选中节点挂到另一节点下（拖拽之外的可靠改层级方式）
  const [moveState, setMoveState] = useState<{ nodeId: string } | null>(null);
  const [moveTargetId, setMoveTargetId] = useState<string | null>(null);

  // 导航树右键菜单
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; nodeId: string } | null>(
    null,
  );
  const treeBoxRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dispatch(loadTemplates());
  }, [dispatch]);

  // 模板列表加载后，若尚未选中任何模板则自动选第一个。
  useEffect(() => {
    if (!activeId && templates.length > 0) {
      setActiveId(templates[0].id);
      dispatch(loadTemplateFull(templates[0].id));
    }
  }, [templates, activeId, dispatch]);

  function openTemplate(id: string) {
    setActiveId(id);
    setSelNodeId(null);
    dispatch(loadTemplateFull(id));
  }

  const nodes = current?.nodes ?? [];
  const edges = current?.edges ?? [];
  const nodeById = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  const { children, roots } = useMemo(() => childrenMap(nodes, edges), [nodes, edges]);
  const parentMap = useMemo(() => containsParentMap(edges), [edges]);

  async function reload() {
    if (activeId) await dispatch(loadTemplateFull(activeId));
  }

  // 把一组兄弟按给定顺序重编号 0..n-1 落库（sort_key 只在同父组内比较）。
  async function reorderGroup(ids: string[]) {
    if (!activeId) return;
    await dispatch(
      reorderNodes({
        template_id: activeId,
        orders: ids.map((id, i) => ({ id, sort_key: i })),
      }),
    ).unwrap();
    await reload();
  }

  // 把 nodeId 挂到 newParentId 下（末尾），并落库其新兄弟顺序。拖拽改层级与「移到…下面」共用。
  async function reparent(nodeId: string, newParentId: string) {
    if (!activeId) return;
    const oldParentId = parentMap[nodeId] ?? null;
    if (oldParentId === newParentId) {
      await reorderGroup([...(children[newParentId] ?? []), nodeId]);
      return;
    }
    const oldEdge = edges.find(
      (e) => e.edge_type === 'contains' && e.target_node_id === nodeId,
    );
    if (oldEdge) {
      await dispatch(deleteEdge({ template_id: activeId, edge_id: oldEdge.id })).unwrap();
    }
    await dispatch(
      createEdge({
        template_id: activeId,
        source_node_id: newParentId,
        target_node_id: nodeId,
      }),
    ).unwrap();
    await reorderGroup([...(children[newParentId] ?? []), nodeId]);
  }

  // ---- 模板增删 ----
  async function submitCreate() {
    const v = await createForm.validateFields();
    try {
      const created = await dispatch(
        createTemplate({ name: v.name, kind: v.kind }),
      ).unwrap();
      setCreateOpen(false);
      createForm.resetFields();
      await dispatch(loadTemplates());
      openTemplate(created.id);
      message.success('模板已创建');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  // ---- 节点增删改 ----
  async function doAdd(type: TemplateNodeType, title: string) {
    if (!activeId || !addState) return;
    try {
      const created = await dispatch(
        createNode({ template_id: activeId, node_type: type, title: title.trim() }),
      ).unwrap();
      if (addState.mode === 'root') {
        await reorderGroup([...roots, created.id]);
      } else if (addState.mode === 'child' && addState.refId) {
        await dispatch(
          createEdge({
            template_id: activeId,
            source_node_id: addState.refId,
            target_node_id: created.id,
          }),
        ).unwrap();
        await reorderGroup([...(children[addState.refId] ?? []), created.id]);
      } else if (addState.mode === 'sibling' && addState.refId) {
        const parentId = parentMap[addState.refId] ?? null;
        if (parentId) {
          await dispatch(
            createEdge({
              template_id: activeId,
              source_node_id: parentId,
              target_node_id: created.id,
            }),
          ).unwrap();
        }
        const sibs = parentId ? children[parentId] ?? [] : roots;
        const idx = sibs.indexOf(addState.refId);
        const next = [...sibs];
        next.splice(idx + 1, 0, created.id);
        await reorderGroup(next);
      }
      message.success('节点已添加');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  async function submitAdd() {
    const v = await addForm.validateFields();
    await doAdd(v.node_type, v.title);
    setAddState(null);
    addForm.resetFields();
  }

  function openAdd(mode: AddMode, refId: string | null) {
    setAddState({ mode, refId });
    addForm.setFieldsValue({ node_type: 'chapter', title: '' });
  }

  function openRename(id: string) {
    setRenameId(id);
    setRenameTitle(nodeById.get(id)?.title ?? '');
  }

  async function submitRename() {
    if (!activeId || !renameId) return;
    const title = renameTitle.trim();
    if (!title) {
      message.warning('标题不能为空');
      return;
    }
    try {
      await dispatch(
        updateNode({ template_id: activeId, node_id: renameId, title }),
      ).unwrap();
      setRenameId(null);
      await reload();
      message.success('已重命名');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  function confirmDelete(id: string) {
    const n = nodeById.get(id);
    modal.confirm({
      title: `删除节点「${n?.title ?? id}」？`,
      content: '其下一级子节点会随边级联脱离，提升为一级节点（文档 root 的子节点）。',
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        if (!activeId) return;
        try {
          await dispatch(deleteNode({ template_id: activeId, node_id: id })).unwrap();
          if (selNodeId === id) setSelNodeId(null);
          await reload();
          message.success('节点已删除');
        } catch (e) {
          message.error(errMsg(e));
        }
      },
    });
  }

  // ---- 「移到…下面」：把选中节点挂到另一节点下（拖拽之外的可靠改层级方式） ----
  function openMove(nodeId: string) {
    setMoveState({ nodeId });
    setMoveTargetId(null);
  }

  const moveCandidates = useMemo(() => {
    if (!moveState) return [];
    const excluded = new Set<string>([moveState.nodeId]);
    const stack = [...(children[moveState.nodeId] ?? [])];
    while (stack.length) {
      const id = stack.pop()!;
      excluded.add(id);
      stack.push(...(children[id] ?? []));
    }
    const build = (ids: string[]): MoveTreeNode[] =>
      ids
        .filter((id) => !excluded.has(id))
        .map((id) => ({
          value: id,
          title: nodeById.get(id)?.title ?? '（未命名）',
          children: build(children[id] ?? []),
        }));
    return build(roots);
  }, [moveState, children, roots, nodeById]);

  async function submitMove() {
    if (!moveState || !moveTargetId) {
      message.warning('请选择要挂到哪个节点下');
      return;
    }
    try {
      await reparent(moveState.nodeId, moveTargetId);
      setMoveState(null);
      setMoveTargetId(null);
      message.success('已移动');
    } catch (e) {
      message.error(errMsg(e));
    }
  }

  // 导航树节点右键 → 在鼠标处弹出操作菜单
  function openCtxMenu(event: React.MouseEvent, nodeId: string) {
    event.preventDefault();
    const rect = treeBoxRef.current?.getBoundingClientRect();
    if (!rect) return;
    const pos = clampMenuPos(event.clientX, event.clientY, rect, 168, 224);
    setSelNodeId(nodeId);
    setCtxMenu({ x: pos.x, y: pos.y, nodeId });
  }

  // ---- 拖拽改层级 / 重排（Word 导航窗格式） ----
  function isDescendant(descId: string, ancId: string): boolean {
    const stack = [...(children[ancId] ?? [])];
    while (stack.length) {
      const id = stack.pop()!;
      if (id === descId) return true;
      stack.push(...(children[id] ?? []));
    }
    return false;
  }

  function computeDrop(
    dragId: string,
    info: TreeDropInfo,
  ): { newParentId: string | null; order: string[] } {
    const targetId = String(info.node.key);
    if (!info.dropToGap) {
      // 拖到某节点上 → 成为它的（最后一个）子节点
      const sibs = (children[targetId] ?? []).filter((id) => id !== dragId);
      sibs.push(dragId);
      return { newParentId: targetId, order: sibs };
    }
    // 拖到缝隙 → 成为目标同级的兄弟，插入目标前/后
    const targetParentId = parentMap[targetId] ?? null;
    const targetSibs = targetParentId ? children[targetParentId] ?? [] : roots;
    const listWithoutDrag = targetSibs.filter((id) => id !== dragId);
    const targetIndexOrig = targetSibs.indexOf(targetId);
    const targetIndexNoDrag = listWithoutDrag.indexOf(targetId);
    const rel = info.dropPosition - targetIndexOrig;
    const insertIdx = rel === -1 ? targetIndexNoDrag : targetIndexNoDrag + 1;
    const clamped = Math.max(0, Math.min(insertIdx, listWithoutDrag.length));
    listWithoutDrag.splice(clamped, 0, dragId);
    return { newParentId: targetParentId, order: listWithoutDrag };
  }

  const onTreeDrop = useCallback(
    async (info: TreeDropInfo) => {
      if (!activeId) return;
      const dragId = String(info.dragNode.key);
      const targetId = String(info.node.key);
      if (dragId === targetId) return;
      if (isDescendant(targetId, dragId)) {
        message.warning('不能把节点挂到它自己的下一级子节点下');
        return;
      }
      const { newParentId, order } = computeDrop(dragId, info);
      const oldParentId = parentMap[dragId] ?? null;
      try {
        if (oldParentId !== newParentId) {
          const oldEdge = edges.find(
            (e) => e.edge_type === 'contains' && e.target_node_id === dragId,
          );
          if (oldEdge) {
            await dispatch(
              deleteEdge({ template_id: activeId, edge_id: oldEdge.id }),
            ).unwrap();
          }
          if (newParentId) {
            await dispatch(
              createEdge({
                template_id: activeId,
                source_node_id: newParentId,
                target_node_id: dragId,
              }),
            ).unwrap();
          }
        }
        await reorderGroup(order);
        message.success('已调整层级');
      } catch (e) {
        message.error(errMsg(e));
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [activeId, parentMap, edges, children, roots, dispatch, message],
  );

  // 画布同组纵向拖拽松手 → 该组新顺序
  const onCanvasReorder = useCallback(
    async (ids: string[]) => {
      try {
        await reorderGroup(ids);
      } catch (e) {
        message.error(errMsg(e));
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [activeId, dispatch, message],
  );

  // ---- 导航窗格树数据 ----
  const nodeTitle = useCallback(
    (id: string) => {
      const n = nodeById.get(id);
      const color = n ? NODE_TYPE_COLOR[n.node_type] : '#bfbfbf';
      return (
        <span className="inline-flex max-w-full items-center gap-1.5">
          <span
            className="h-2 w-2 flex-none rounded-[2px]"
            style={{ background: color }}
          />
          <span className="truncate">
            {n?.title ?? '（未命名）'}
          </span>
        </span>
      );
    },
    [nodeById],
  );

  const treeData = useMemo<TreeDataNode[]>(() => {
    const seen = new Set<string>();
    const build = (ids: string[]): TreeDataNode[] =>
      ids.map((id) => {
        seen.add(id);
        return {
          key: id,
          title: nodeTitle(id),
          children: build(children[id] ?? []),
        };
      });
    const data = build(roots);
    // 兜底：未被任何 root 到达的节点（环）补为一级
    for (const n of nodes) {
      if (!seen.has(n.id)) {
        data.push({ key: n.id, title: nodeTitle(n.id), children: [] });
      }
    }
    return data;
  }, [roots, children, nodes, nodeTitle]);

  return (
    <Space direction="vertical" className="w-full!" size={12}>
      <Space wrap>
        <Select
          className="w-[280px]!"
          placeholder="选择要编辑的文档模板"
          value={activeId}
          onChange={openTemplate}
          options={templates.map((t: DocumentTemplate) => ({
            value: t.id,
            label: `${t.name}（${t.kind}）`,
          }))}
        />
        <Button onClick={() => activeId && dispatch(loadTemplateFull(activeId))} loading={loading}>
          刷新
        </Button>
        <Button type="primary" onClick={() => setCreateOpen(true)}>
          新建模板
        </Button>
      </Space>

      <ErrorAlert error={error} />

      {!current && !loading && (
        <Empty description="选择或新建一份文档模板，左侧搭骨架，右侧看结构。" />
      )}

      {current && (
        <Tabs
          defaultActiveKey="structure"
          items={[
            {
              key: 'structure',
              label: '1. 文档结构',
              children: (
                <Layout className="h-[calc(100vh-266px)]">
                  <Sider width={320} theme="light" className="overflow-auto! p-2!">
                    <Space direction="vertical" className="w-full!" size={8}>
                      <Space wrap>
                        <Typography.Text strong>{current.template.name}</Typography.Text>
                        <Tag>{current.template.kind}</Tag>
                      </Space>
                      <Typography.Text type="secondary" className="text-xs">
                        导航窗格：拖到缝隙=同级重排；拖到节点上并略向右=设为子节点；右键节点=更多操作。
                      </Typography.Text>
                      <Divider className="mb-1! mt-1!" />
                      {/* 操作区 */}
                      <Space direction="vertical" className="w-full!" size={6}>
                        <Button block type="primary" onClick={() => openAdd('root', null)}>
                          ＋ 新增一级节点
                        </Button>
                      </Space>
                      <Divider className="mb-1! mt-1!" />
                      <div ref={treeBoxRef} className="relative">
                        <Tree
                          key={activeId}
                          draggable
                          blockNode
                          defaultExpandAll
                          treeData={treeData}
                          selectedKeys={selNodeId ? [selNodeId] : []}
                          onSelect={(keys) =>
                            setSelNodeId(keys.length ? String(keys[0]) : null)
                          }
                          onDrop={onTreeDrop}
                          onRightClick={({ event, node }) =>
                            openCtxMenu(event, String(node.key))
                          }
                          className="bg-transparent!"
                        />
                        {ctxMenu && (
                          <ContextMenu
                            title={nodeById.get(ctxMenu.nodeId)?.title ?? '节点'}
                            x={ctxMenu.x}
                            y={ctxMenu.y}
                            onClose={() => setCtxMenu(null)}
                            items={[
                              { key: 'child', label: '新增子节点', onClick: () => openAdd('child', ctxMenu.nodeId) },
                              { key: 'sibling', label: '新增同级', onClick: () => openAdd('sibling', ctxMenu.nodeId) },
                              { key: 'move', label: '移到…下面', onClick: () => openMove(ctxMenu.nodeId) },
                              { key: 'rename', label: '重命名', onClick: () => openRename(ctxMenu.nodeId) },
                              { key: 'delete', label: '删除', danger: true, onClick: () => confirmDelete(ctxMenu.nodeId) },
                            ]}
                          />
                        )}
                      </div>
                    </Space>
                  </Sider>
                  <Content className="relative">
                    <div className="absolute inset-0">
                      <TemplateCanvas
                        nodes={nodes}
                        edges={edges}
                        selectedId={selNodeId}
                        onSelectNode={(id) => setSelNodeId(id)}
                        onReorder={onCanvasReorder}
                      />
                    </div>
                  </Content>
                </Layout>
              ),
            },
            {
              key: 'content',
              label: '2. 编排内容',
              children: (
                <ContentComposer
                  templateId={current.template.id}
                  nodes={nodes}
                  edges={edges}
                  selectedId={selNodeId}
                  onSelect={setSelNodeId}
                />
              ),
            },
            {
              key: 'preview',
              label: '3. 装配预览',
              children: (
                <AssemblePreview
                  templateId={current.template.id}
                  templateName={current.template.name}
                  templateKind={current.template.kind}
                />
              ),
            },
          ]}
        />
      )}

      {/* 新建模板 */}
      <Modal
        title="新建文档模板"
        open={createOpen}
        onOk={submitCreate}
        onCancel={() => setCreateOpen(false)}
        okText="创建"
        cancelText="取消"
      >
        <Form form={createForm} layout="vertical" initialValues={{ kind: '大纲' }}>
          <Form.Item
            name="name"
            label="模板名称"
            rules={[{ required: true, message: '请输入模板名称' }]}
          >
            <Input placeholder="如：测试大纲 / 测试报告" />
          </Form.Item>
          <Form.Item name="kind" label="类别（对应 ConTeXt kind）">
            <Select options={KIND_OPTIONS.map((k) => ({ value: k, label: k }))} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 新增节点 */}
      <Modal
        title={addState ? ADD_MODE_TITLE[addState.mode] : '新增节点'}
        open={addState !== null}
        onOk={submitAdd}
        onCancel={() => setAddState(null)}
        okText="添加"
        cancelText="取消"
      >
        <Form form={addForm} layout="vertical" initialValues={{ node_type: 'chapter' }}>
          <Form.Item name="node_type" label="节点类型">
            <Select options={TYPE_OPTIONS} />
          </Form.Item>
          <Form.Item
            name="title"
            label="标题"
            rules={[{ required: true, message: '请输入节点标题' }]}
          >
            <Input placeholder="如：1 概述" onPressEnter={submitAdd} />
          </Form.Item>
        </Form>
      </Modal>

      {/* 重命名 */}
      <Modal
        title="重命名节点"
        open={renameId !== null}
        onOk={submitRename}
        onCancel={() => setRenameId(null)}
        okText="保存"
        cancelText="取消"
      >
        <Input
          value={renameTitle}
          onChange={(e) => setRenameTitle(e.target.value)}
          onPressEnter={submitRename}
          placeholder="节点标题"
        />
      </Modal>

      {/* 移到…下面 */}
      <Modal
        title={`把「${nodeById.get(moveState?.nodeId ?? '')?.title ?? ''}」挂到…下面`}
        open={moveState !== null}
        onOk={submitMove}
        onCancel={() => setMoveState(null)}
        okText="移动"
        cancelText="取消"
        okButtonProps={{ disabled: !moveTargetId }}
      >
        <Typography.Text type="secondary" className="text-xs">
          选择新的父节点（已排除自身及其子节点）：
        </Typography.Text>
        <TreeSelect
          className="mt-2! w-full!"
          value={moveTargetId}
          onChange={(v) => setMoveTargetId(v)}
          treeData={moveCandidates}
          treeDefaultExpandAll
          placeholder="选择要挂到哪个节点下"
        />
      </Modal>
    </Space>
  );
}
