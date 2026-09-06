import { useEffect, useRef } from 'react';
import { Button } from 'antd';
import type { ReactNode } from 'react';

export interface ContextMenuItem {
  key: string;
  label: ReactNode;
  danger?: boolean;
  disabled?: boolean;
  loading?: boolean;
  onClick: () => void;
}

interface ContextMenuProps {
  /** 顶部小标题（可选，如「节点标识 · 标题」） */
  title?: ReactNode;
  items: ContextMenuItem[];
  /** 相对定位容器的坐标（用 clampMenuPos 计算） */
  x: number;
  y: number;
  onClose: () => void;
  minWidth?: number;
}

/**
 * 通用右键菜单：绝对定位在「position:relative」的容器内，点击外部自动关闭。
 * 需求追踪画布、文档模板导航窗格等各处共用。
 */
export default function ContextMenu({
  title,
  items,
  x,
  y,
  onClose,
  minWidth = 160,
}: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  // 点击菜单外部（含右键其它地方）时关闭。
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="absolute z-40 select-none rounded-lg border border-neutral-200 bg-white p-1 shadow-[0_3px_12px_rgba(0,0,0,0.18)]"
      style={{ left: x, top: y, minWidth }}
      onMouseDown={(e) => e.stopPropagation()}
      onClick={(e) => e.stopPropagation()}
    >
      {title != null && (
        <div className="mb-1 max-w-[220px] truncate border-b border-neutral-100 px-2.5 py-0.5 text-[11px] text-neutral-500">
          {title}
        </div>
      )}
      {items.map((item) => (
        <Button
          key={item.key}
          type="text"
          size="small"
          block
          danger={item.danger}
          disabled={item.disabled}
          loading={item.loading}
          className="w-full rounded-md text-left"
          onClick={() => {
            onClose();
            item.onClick();
          }}
        >
          {item.label}
        </Button>
      ))}
    </div>
  );
}

/** 把鼠标客户端坐标换算成「相对容器」的坐标，并夹在容器内（避免菜单溢出右/下边界）。 */
export function clampMenuPos(
  clientX: number,
  clientY: number,
  rect: DOMRect,
  menuW: number,
  menuH: number,
): { x: number; y: number } {
  return {
    x: Math.max(0, Math.min(clientX - rect.left, rect.width - menuW)),
    y: Math.max(0, Math.min(clientY - rect.top, rect.height - menuH)),
  };
}
