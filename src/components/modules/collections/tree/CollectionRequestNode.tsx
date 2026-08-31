import React from 'react';
import { MethodBadge } from '../../../common/MethodBadge';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { TreeGuideLines } from './TreeGuideLines';
import type { RequestItem } from '../../../../services/tauri/bridge';

interface CollectionRequestNodeProps {
  request: RequestItem;
  depth: number;
  isLast?: boolean;
  ancestorIsLast?: boolean[];
  isSearching: boolean;
  editingId: string | null;
  editingType: 'collection' | 'request' | null;
  editingName: string;
  onSetEditingName: (name: string) => void;
  onSaveRename: () => void;
  onCancelRename: () => void;
  onOpenRequestTab: (req: RequestItem) => void;
  onContextMenu: (e: React.MouseEvent, type: 'request', id: string, item: RequestItem) => void;
  onDragStart: (e: React.DragEvent, req: RequestItem) => void;
  onDeleteRequest: (id: string) => void;
}

export const CollectionRequestNode: React.FC<CollectionRequestNodeProps> = ({
  request,
  depth,
  isLast = false,
  ancestorIsLast = [],
  isSearching,
  editingId,
  editingType,
  editingName,
  onSetEditingName,
  onSaveRename,
  onCancelRename,
  onOpenRequestTab,
  onContextMenu,
  onDragStart,
  onDeleteRequest,
}) => {
  const isRenaming = editingId === request.id && editingType === 'request';

  return (
    <div
      draggable
      onDragStart={(e) => onDragStart(e, request)}
      onClick={() => onOpenRequestTab(request)}
      onContextMenu={(e) => onContextMenu(e, 'request', request.id, request)}
      className="group flex items-center justify-between pr-2 rounded-md hover:bg-neutral-subtle/80 cursor-pointer font-sans text-xs transition-colors select-none h-7"
      style={{ paddingLeft: depth === 0 ? '6px' : '0px' }}
    >
      <div className="flex items-center gap-1.5 truncate min-w-0 flex-1 self-stretch h-full">
        {/* Tree Structure Guide Lines */}
        <TreeGuideLines depth={depth} isLast={isLast} ancestorIsLast={ancestorIsLast} />

        <MethodBadge method={request.method} className="ml-0.5 shrink-0" />
        {isRenaming ? (
          <input
            type="text"
            autoFocus
            value={editingName}
            onChange={(e) => onSetEditingName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') onSaveRename();
              if (e.key === 'Escape') onCancelRename();
            }}
            onBlur={onSaveRename}
            onClick={(e) => e.stopPropagation()}
            className="bg-background border border-primary rounded px-1 py-0.5 text-xs text-foreground font-sans w-full focus:outline-none"
          />
        ) : (
          <div className="flex flex-col min-w-0 truncate">
            <span className="truncate text-foreground font-medium">{request.name}</span>
            {isSearching && request.url && (
              <span className="truncate text-[10px] text-muted-foreground font-mono" title={request.url}>
                {request.url}
              </span>
            )}
          </div>
        )}
      </div>

      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onDeleteRequest(request.id);
        }}
        className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 p-0.5 rounded transition-opacity shrink-0 cursor-pointer"
        title="Delete Request"
      >
        <MingCuteIcon name="close_line" size={13} />
      </button>
    </div>
  );
};
