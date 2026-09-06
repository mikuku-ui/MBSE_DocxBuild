// 顶栏右上项目**快捷切换器**：纯切换当前项目。
// 项目的新增 / 重命名 / 删除在「项目管理」页（ProjectPage）处理；这里只负责快速
// 切换当前项目 + 记忆最近选中项。切换后各模块按 `currentProjectId` 收口数据。

import { useEffect } from 'react';
import { App, Button, Dropdown, Space, Typography } from 'antd';
import type { MenuProps } from 'antd';
import { useAppDispatch, useAppSelector } from '../../app/hooks';
import { loadProjects, setCurrentProjectId } from './projectSlice';

const LS_KEY = 'mbse.trace.currentProjectId';

function readSaved(): string | null {
  try {
    return localStorage.getItem(LS_KEY);
  } catch {
    return null;
  }
}

function persistSaved(id: string | null) {
  try {
    if (id) localStorage.setItem(LS_KEY, id);
    else localStorage.removeItem(LS_KEY);
  } catch {
    /* 忽略（隐私模式等） */
  }
}

export default function ProjectSwitcher() {
  const dispatch = useAppDispatch();
  const { message } = App.useApp();
  const { projects, currentProjectId, loading } = useAppSelector((s) => s.projects);

  // 挂载拉一次项目列表（新增/重命名/删除后项目管理页与这里共用 Redux，自动同步）
  useEffect(() => {
    dispatch(loadProjects());
  }, [dispatch]);

  // 首次 / 当前项目已失效时：回落到 localStorage 记录的项目，否则取第一个。
  useEffect(() => {
    if (projects.length === 0) return;
    if (currentProjectId && projects.some((p) => p.id === currentProjectId)) return;
    const saved = readSaved();
    const target =
      saved && projects.some((p) => p.id === saved) ? saved : projects[0].id;
    dispatch(setCurrentProjectId(target));
  }, [projects, currentProjectId, dispatch]);

  // 当前项目变化即记忆，下次打开仍停在那个项目。
  useEffect(() => {
    persistSaved(currentProjectId);
  }, [currentProjectId]);

  const current = projects.find((p) => p.id === currentProjectId) ?? null;

  const menuItems: MenuProps['items'] =
    projects.length === 0
      ? [{ key: '__empty__', label: '暂无项目，请到「项目管理」页新建' }]
      : projects.map((p) => ({
          key: p.id,
          label: (
            <span>
              {p.name}
              {p.id === currentProjectId ? (
                <Typography.Text type="secondary" className="ml-1.5">
                  ✓
                </Typography.Text>
              ) : null}
            </span>
          ),
        }));

  function onMenuClick({ key }: { key: string }) {
    if (projects.some((p) => p.id === key)) {
      dispatch(setCurrentProjectId(key));
    } else if (projects.length === 0) {
      message.info('项目的新增 / 重命名 / 删除在左侧「项目管理」页处理');
    }
  }

  return (
    <Dropdown
      menu={{ items: menuItems, onClick: onMenuClick }}
      trigger={['click']}
      placement="bottomRight"
    >
      <Button
        type="text"
        className="h-[34px]! px-2.5! text-white!"
        loading={loading}
        title="快捷切换当前项目（项目的新增/重命名/删除在「项目管理」页）"
      >
        <Space size={5}>
          <span className="max-w-[220px] truncate font-semibold">
            {current ? current.name : '选择项目'}
          </span>
          <span className="text-[10px] opacity-80">▾</span>
        </Space>
      </Button>
    </Dropdown>
  );
}
