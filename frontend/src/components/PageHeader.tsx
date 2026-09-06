import { Typography } from 'antd';
import type { ReactNode } from 'react';

interface PageHeaderProps {
  title: ReactNode;
  /** 标题下方的小字说明（可选） */
  description?: ReactNode;
  /** 右侧操作区（如「＋ 新增」按钮） */
  extra?: ReactNode;
}

/** 各页通用的页头：左 = 标题 + 说明，右 = 操作区，两端对齐。 */
export default function PageHeader({ title, description, extra }: PageHeaderProps) {
  return (
    <div className="flex w-full flex-wrap items-center justify-between gap-2">
      <div>
        <Typography.Title level={5} className="mb-0!">
          {title}
        </Typography.Title>
        {description != null && (
          <Typography.Text type="secondary" className="block text-xs leading-5">
            {description}
          </Typography.Text>
        )}
      </div>
      {extra}
    </div>
  );
}
