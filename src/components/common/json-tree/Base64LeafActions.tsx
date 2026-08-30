import React, { useState, useMemo } from 'react';
import { detectBase64, saveBase64ToFile } from '../../../utils/base64Helper';
import { Base64PreviewModal } from '../Base64PreviewModal';
import { MingCuteIcon } from '../MingCuteIcon';

interface Base64LeafActionsProps {
  value: string;
  label?: string;
}

export const Base64LeafActions: React.FC<Base64LeafActionsProps> = ({ value, label }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [showInline, setShowInline] = useState(false);
  const info = useMemo(() => detectBase64(value, 200), [value]);

  if (!info) return null;

  const dataUri = `data:${info.mimeType};base64,${info.cleanB64}`;

  const handleSave = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const name = label ? `${label}.${info.extension}` : `file.${info.extension}`;
    await saveBase64ToFile(info.cleanB64, name, info.mimeType);
  };

  const isMedia =
    info.previewType === 'image' ||
    info.previewType === 'audio' ||
    info.previewType === 'video' ||
    info.previewType === 'pdf';

  return (
    <div className="inline-flex flex-col gap-1 align-middle my-0.5">
      <span className="inline-flex items-center gap-1.5 ml-2 select-none shrink-0 font-mono text-[10px]">
        <span className="font-bold text-amber-400 bg-amber-500/10 border border-amber-500/30 px-1.5 py-0.5 rounded flex items-center gap-1">
          ⚡ Base64 ({info.extension.toUpperCase()})
        </span>

        {isMedia && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setShowInline(!showInline);
            }}
            className={`font-bold px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1 border ${
              showInline
                ? 'bg-purple-500/20 text-purple-300 border-purple-500/40'
                : 'bg-background text-muted-foreground border-border hover:text-foreground'
            }`}
            title="Toggle Inline Live Preview"
          >
            <span>{showInline ? '▼ Hide' : '▶ Live Preview'}</span>
          </button>
        )}

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setIsOpen(true);
          }}
          className="font-bold text-sky-400 bg-sky-500/10 border border-sky-500/30 hover:bg-sky-500/20 px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1"
          title="Open Full Preview Modal"
        >
          <MingCuteIcon name="eye_line" size={11} />
          <span>Open</span>
        </button>

        <button
          type="button"
          onClick={handleSave}
          className="font-bold text-emerald-400 bg-emerald-500/10 border border-emerald-500/30 hover:bg-emerald-500/20 px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1"
          title="Save Decoded File"
        >
          <MingCuteIcon name="download_line" size={11} />
          <span>Save</span>
        </button>
      </span>

      {showInline && isMedia && (
        <div className="my-1.5 ml-2 p-2 bg-background border border-border rounded-lg shadow-inner flex flex-col gap-2 max-w-md animate-in fade-in duration-150 font-mono text-[10px]">
          <div className="flex items-center justify-between text-muted-foreground border-b border-border pb-1">
            <span className="font-bold text-foreground">Live Preview ({info.mimeType})</span>
            <button
              type="button"
              onClick={() => setIsOpen(true)}
              className="text-primary hover:underline cursor-pointer"
            >
              Full Screen ↗
            </button>
          </div>

          {info.previewType === 'image' && (
            <img
              src={dataUri}
              alt="Live Base64 Preview"
              className="max-h-48 max-w-full object-contain rounded border border-border bg-surface p-1 cursor-pointer"
              onClick={() => setIsOpen(true)}
              title="Click for full view"
            />
          )}

          {info.previewType === 'audio' && <audio controls src={dataUri} className="w-full h-8" />}

          {info.previewType === 'video' && (
            <video controls src={dataUri} className="max-h-48 max-w-full rounded border border-border" />
          )}

          {info.previewType === 'pdf' && (
            <div className="flex items-center gap-2 p-2 bg-surface rounded border border-border">
              <span className="text-rose-400 font-bold text-xs">📄 PDF Document</span>
              <button
                type="button"
                onClick={() => setIsOpen(true)}
                className="px-2 py-0.5 font-bold text-primary bg-primary/10 border border-primary/30 rounded cursor-pointer"
              >
                Open Viewer
              </button>
            </div>
          )}
        </div>
      )}

      <Base64PreviewModal
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        data={value}
        fieldName={label || 'payload'}
      />
    </div>
  );
};
