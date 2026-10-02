import React from 'react';
import { Select, Button } from '../../../common/ui';
import { MingCuteIcon } from '../../../common/MingCuteIcon';

export const HTTP_METHODS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
] as const;

interface CollectionAddressBarProps {
  method: string;
  url: string;
  isLoading: boolean;
  onMethodChange: (method: string) => void;
  onUrlChange: (url: string) => void;
  onUrlBlur: () => void;
  onSend: () => void;
  isHistoryOpen: boolean;
  onToggleHistory: () => void;
}

export const CollectionAddressBar: React.FC<CollectionAddressBarProps> = ({
  method,
  url,
  isLoading,
  onMethodChange,
  onUrlChange,
  onUrlBlur,
  onSend,
  isHistoryOpen,
  onToggleHistory,
}) => {
  return (
    <div className="h-12 px-2.5 bg-header border-b border-border flex items-center gap-2 shrink-0 font-mono">
      <div className="w-28 shrink-0 font-bold">
        <Select
          value={method}
          onChange={(e) => onMethodChange(e.target.value)}
          options={HTTP_METHODS}
        />
      </div>

      <div className="flex-1 relative flex items-center">
        <input
          type="text"
          value={url}
          onChange={(e) => onUrlChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              onSend();
            }
          }}
          onBlur={onUrlBlur}
          placeholder="https://api.example.com/v1/resource or {{BASE_URL}}/endpoint"
          className="w-full bg-background border border-border rounded-md px-3 py-1.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary shadow-2xs"
        />
      </div>

      <Button
        variant="primary"
        icon="send_plane_line"
        onClick={onSend}
        disabled={isLoading}
        className="shadow-2xs font-semibold px-4 py-1.5 shrink-0"
      >
        {isLoading ? 'Sending...' : 'Send'}
      </Button>

      <button
        onClick={onToggleHistory}
        className={`p-1.5 rounded border cursor-pointer transition-colors shrink-0 ${
          isHistoryOpen
            ? 'bg-primary/10 border-primary/40 text-primary'
            : 'bg-background border-border text-muted-foreground hover:text-foreground'
        }`}
        title={isHistoryOpen ? 'Hide Run History' : 'Show Run History'}
      >
        <MingCuteIcon name="history_line" size={16} />
      </button>
    </div>
  );
};
