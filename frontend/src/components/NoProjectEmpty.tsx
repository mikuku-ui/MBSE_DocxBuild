import { Empty } from 'antd';
import type { ReactNode } from 'react';

interface NoProjectEmptyProps {
  description: ReactNode;
  /** true = 铺满可用高度居中（整页引导，如追踪页）；false = 顶部留白（内容区内引导） */
  center?: boolean;
  /** Empty 内部的补充说明（可选） */
  children?: ReactNode;
}

/** 「尚未选择项目」的空态引导：数据按项目隔离的页面在 currentProjectId 为空时共用。 */
export default function NoProjectEmpty({
  description,
  center = false,
  children,
}: NoProjectEmptyProps) {
  if (center) {
    return (
      <div className="flex h-[calc(100vh-128px)] items-center justify-center">
        <Empty description={description}>{children}</Empty>
      </div>
    );
  }
  return (
    <Empty className="mt-20" description={description}>
      {children}
    </Empty>
  );
}
