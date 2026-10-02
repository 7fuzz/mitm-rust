import React, { useState } from 'react';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import type { RequestPreview } from '../../../../services/tauri/bridge';

interface CollectionInterpolationTabProps {
  preview: RequestPreview | null;
  isLoading: boolean;
  error: string | null;
  onRefresh: () => void;
}

export const CollectionInterpolationTab: React.FC<CollectionInterpolationTabProps> = ({
  preview,
  isLoading,
  error,
  onRefresh,
}) => {
  const [previewMode, setPreviewMode] = useState<'full_url' | 'host' | 'curl'>('host');

  const getActiveText = () => {
    if (!preview) return '';
    if (previewMode === 'full_url') return preview.fullUrlRequest;
    if (previewMode === 'host') return preview.fullRequest;
    return preview.curlCommand;
  };

  return (
    <div className="h-full flex flex-col gap-0 font-mono text-xs">
      {/* Mode switcher + refresh */}
      <div className="flex items-center justify-between pb-2 shrink-0">
        <div className="flex items-center gap-1 bg-background border border-border rounded-lg p-0.5">
          {[
            { id: 'host', label: 'request' },
            { id: 'full_url', label: 'custom' },
            { id: 'curl', label: 'cURL' },
          ].map((m) => (
            <button
              key={m.id}
              type="button"
              onClick={() => setPreviewMode(m.id as 'full_url' | 'host' | 'curl')}
              className={`px-3 py-1 rounded text-2xs font-semibold transition-colors cursor-pointer ${
                previewMode === m.id
                  ? 'bg-primary text-primary-foreground shadow'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {m.label}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={onRefresh}
          className="flex items-center gap-1 px-2 py-1 rounded text-2xs border border-border bg-background hover:bg-neutral-subtle transition-colors cursor-pointer text-muted-foreground hover:text-foreground"
          title="Refresh preview"
        >
          <MingCuteIcon name="refresh_1_line" size={12} />
          <span>Refresh</span>
        </button>
      </div>

      {/* Content area */}
      {isLoading ? (
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          <MingCuteIcon name="loading_3_line" size={20} className="animate-spin mr-2" />
          <span>Resolving variables...</span>
        </div>
      ) : error ? (
        <div className="flex-1 flex items-center justify-center text-rose-400 text-xs">
          <MingCuteIcon name="alert_circle_line" size={16} className="mr-2" />
          <span>{error}</span>
        </div>
      ) : preview ? (
        <div className="flex-1 border border-border rounded-lg overflow-hidden bg-background relative">
          <button
            type="button"
            onClick={() => {
              navigator.clipboard.writeText(getActiveText());
            }}
            className="absolute top-2 right-2 z-10 flex items-center gap-1 px-2 py-0.5 rounded text-3xs bg-surface border border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-colors cursor-pointer"
            title="Copy to clipboard"
          >
            <MingCuteIcon name="copy_2_line" size={11} />
            <span>Copy</span>
          </button>
          <pre className="p-4 overflow-auto h-full text-2xs leading-relaxed whitespace-pre-wrap break-all text-foreground font-mono">
            {getActiveText()}
          </pre>
        </div>
      ) : (
        <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground italic gap-2">
          <MingCuteIcon name="eye_2_line" size={32} className="opacity-30" />
          <span className="text-xs">Click to load preview</span>
        </div>
      )}
    </div>
  );
};
