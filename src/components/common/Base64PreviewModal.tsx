import React, { useMemo } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { Button } from './ui/Button';
import { detectBase64, saveBase64ToFile } from '../../utils/base64Helper';
import { CodeEditor } from './CodeEditor';

interface Base64PreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  data: string;
  fieldName?: string;
}

export const Base64PreviewModal: React.FC<Base64PreviewModalProps> = ({
  isOpen,
  onClose,
  data,
  fieldName = 'base64_payload',
}) => {
  const info = useMemo(() => detectBase64(data, 16), [data]);

  if (!isOpen || !info) return null;

  const dataUri = `data:${info.mimeType};base64,${info.cleanB64}`;

  const handleSave = async () => {
    const fileName = `${fieldName}.${info.extension}`;
    await saveBase64ToFile(info.cleanB64, fileName, info.mimeType);
  };

  const renderContent = () => {
    if (info.previewType === 'image') {
      return (
        <div className="flex flex-col items-center justify-center gap-3 p-4 bg-background rounded-lg border border-border">
          <img
            src={dataUri}
            alt="Base64 Preview"
            className="max-h-[60vh] max-w-full object-contain rounded border border-border p-2 bg-surface"
          />
        </div>
      );
    } else if (info.previewType === 'pdf') {
      return (
        <div className="h-[60vh] w-full border border-border rounded-lg overflow-hidden bg-white">
          <iframe src={dataUri} className="w-full h-full" title="Base64 PDF Preview" />
        </div>
      );
    } else if (info.previewType === 'audio') {
      return (
        <div className="p-6 bg-background rounded-lg border border-border flex flex-col items-center gap-4">
          <MingCuteIcon name="radio_line" size={40} className="text-amber-500" />
          <audio controls src={dataUri} className="w-full max-w-md" />
        </div>
      );
    } else if (info.previewType === 'video') {
      return (
        <div className="p-4 bg-background rounded-lg border border-border flex justify-center">
          <video controls src={dataUri} className="max-h-[60vh] max-w-full rounded" />
        </div>
      );
    } else if (info.previewType === 'json' || info.previewType === 'text') {
      try {
        const decoded = atob(info.cleanB64);
        if (info.previewType === 'json') {
          const parsed = JSON.parse(decoded);
          return <CodeEditor value={JSON.stringify(parsed, null, 2)} language="json" readOnly />;
        }
        return <CodeEditor value={decoded} language="plaintext" readOnly />;
      } catch {
        return <CodeEditor value={info.cleanB64} language="plaintext" readOnly />;
      }
    }

    return (
      <div className="p-8 text-center text-muted-foreground italic bg-background rounded-lg border border-border flex flex-col items-center gap-2 font-mono">
        <MingCuteIcon name="storage_line" size={36} className="opacity-40" />
        <span>Binary file ({info.mimeType} - {info.sizeBytes} bytes)</span>
      </div>
    );
  };

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 font-sans text-xs select-none cursor-pointer"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="bg-surface border border-border rounded-xl shadow-2xl w-full max-w-2xl p-5 flex flex-col gap-4 text-foreground cursor-default select-text"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border pb-3">
          <div className="flex items-center gap-2 font-semibold text-sm font-mono">
            <MingCuteIcon name="zap_line" size={18} className="text-amber-500" />
            <span>
              Base64 Preview: <span className="text-primary font-bold">{fieldName}</span> ({info.extension.toUpperCase()})
            </span>
          </div>
          <button onClick={onClose} className="p-1 text-muted-foreground hover:text-foreground rounded cursor-pointer">
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {/* Preview Area */}
        <div className="max-h-[70vh] overflow-y-auto">{renderContent()}</div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-border pt-3 font-mono text-[11px]">
          <span className="text-muted-foreground">
            Type: <strong className="text-foreground">{info.mimeType}</strong> ({info.sizeBytes} bytes)
          </span>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={onClose}>
              Close
            </Button>
            <Button variant="success" size="sm" icon="download_line" onClick={handleSave}>
              Save File
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
