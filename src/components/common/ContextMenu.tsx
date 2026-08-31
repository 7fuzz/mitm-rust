import React, { useEffect, useRef } from 'react';
import { MingCuteIcon, MingCuteIconName } from './MingCuteIcon';

export interface ContextMenuItem {
  label: string;
  icon: MingCuteIconName;
  action: () => void;
  danger?: boolean;
}

interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({ x, y, items, onClose }) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  return (
    <div
      ref={menuRef}
      className="fixed z-50 min-w-[180px] bg-surface border border-border rounded-md shadow-xl py-1 text-xs animate-in fade-in zoom-in-95 duration-100"
      style={{ left: `${x}px`, top: `${y}px` }}
    >
      {items.map((item, idx) => (
        <button
          key={idx}
          onClick={() => {
            item.action();
            onClose();
          }}
          className={`w-full px-3 py-1.5 flex items-center gap-2 text-left hover:bg-neutral-subtle transition-colors ${
            item.danger ? 'text-rose-500 hover:bg-rose-500/10' : 'text-foreground'
          }`}
        >
          <MingCuteIcon name={item.icon} size={15} />
          <span className="font-medium">{item.label}</span>
        </button>
      ))}
    </div>
  );
};
