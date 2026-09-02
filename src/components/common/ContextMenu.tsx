import React, { useEffect, useRef, useState } from 'react';
import { MingCuteIcon, MingCuteIconName } from './MingCuteIcon';

export interface ContextMenuItem {
  label: string;
  icon?: MingCuteIconName;
  action?: () => void;
  danger?: boolean;
  disabled?: boolean;
  children?: ContextMenuItem[];
}

interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({ x, y, items, onClose }) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenuIdx, setActiveSubmenuIdx] = useState<number | null>(null);
  const closeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('mousedown', handleClickOutside);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousedown', handleClickOutside);
      window.removeEventListener('keydown', handleKeyDown);
      if (closeTimeoutRef.current) {
        clearTimeout(closeTimeoutRef.current);
      }
    };
  }, [onClose]);

  // Viewport bounds checking
  const menuWidth = 190;
  const menuHeight = items.length * 32 + 10;
  const isNearRight = x + menuWidth + 210 > window.innerWidth;
  const clampedX = Math.min(Math.max(8, x), Math.max(8, window.innerWidth - menuWidth - 8));
  const clampedY = Math.min(Math.max(8, y), Math.max(8, window.innerHeight - menuHeight - 8));

  const handleMouseEnterItem = (idx: number, hasChildren: boolean) => {
    if (closeTimeoutRef.current) {
      clearTimeout(closeTimeoutRef.current);
      closeTimeoutRef.current = null;
    }
    if (hasChildren) {
      setActiveSubmenuIdx(idx);
    } else {
      setActiveSubmenuIdx(null);
    }
  };

  const handleMouseLeaveItem = () => {
    closeTimeoutRef.current = setTimeout(() => {
      setActiveSubmenuIdx(null);
    }, 180);
  };

  return (
    <div
      ref={menuRef}
      className="fixed z-50 min-w-[180px] bg-surface border border-border rounded-md shadow-xl py-1 text-xs animate-in fade-in zoom-in-95 duration-100 select-none font-sans"
      style={{ left: `${clampedX}px`, top: `${clampedY}px` }}
    >
      {items.map((item, idx) => {
        const hasChildren = !!(item.children && item.children.length > 0);
        const isSubmenuOpen = activeSubmenuIdx === idx;

        return (
          <div
            key={idx}
            className="relative"
            onMouseEnter={() => handleMouseEnterItem(idx, hasChildren)}
            onMouseLeave={handleMouseLeaveItem}
          >
            <button
              type="button"
              disabled={item.disabled}
              onClick={() => {
                if (!hasChildren && item.action) {
                  item.action();
                  onClose();
                }
              }}
              className={`w-full px-3 py-1.5 flex items-center gap-2 text-left transition-colors cursor-pointer ${
                item.disabled
                  ? 'opacity-40 cursor-not-allowed'
                  : item.danger
                  ? 'text-rose-500 hover:bg-rose-500/10'
                  : isSubmenuOpen
                  ? 'bg-neutral-subtle text-foreground'
                  : 'text-foreground hover:bg-neutral-subtle'
              }`}
            >
              {item.icon && <MingCuteIcon name={item.icon} size={15} className="shrink-0" />}
              <span className="font-medium flex-1 truncate">{item.label}</span>
              {hasChildren && (
                <MingCuteIcon
                  name="chevron_right_line"
                  size={14}
                  className="ml-auto text-muted-foreground shrink-0"
                />
              )}
            </button>

            {/* Submenu Dropdown */}
            {hasChildren && isSubmenuOpen && (
              <div
                className={`absolute top-0 z-50 min-w-[210px] bg-surface border border-border rounded-md shadow-xl py-1 text-xs animate-in fade-in zoom-in-95 duration-75 ${
                  isNearRight ? 'right-full mr-1' : 'left-full ml-1'
                }`}
                onMouseEnter={() => {
                  if (closeTimeoutRef.current) {
                    clearTimeout(closeTimeoutRef.current);
                    closeTimeoutRef.current = null;
                  }
                }}
              >
                {item.children!.map((child, childIdx) => (
                  <button
                    key={childIdx}
                    type="button"
                    disabled={child.disabled}
                    onClick={() => {
                      if (child.action) {
                        child.action();
                        onClose();
                      }
                    }}
                    className={`w-full px-3 py-1.5 flex items-center gap-2 text-left transition-colors cursor-pointer ${
                      child.disabled
                        ? 'opacity-40 cursor-not-allowed'
                        : child.danger
                        ? 'text-rose-500 hover:bg-rose-500/10'
                        : 'text-foreground hover:bg-neutral-subtle'
                    }`}
                  >
                    {child.icon && <MingCuteIcon name={child.icon} size={15} className="shrink-0" />}
                    <span className="font-medium flex-1">{child.label}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};
