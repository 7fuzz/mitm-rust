import React, { useState } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { Base64PreviewModal } from './Base64PreviewModal';
import { saveBase64ToFile } from '../../utils/base64Helper';
import type { MediaDetectionResult } from '../../utils/mediaDetector';

interface MediaResponsePreviewProps {
  media: MediaDetectionResult;
  title?: string;
}

export const MediaResponsePreview: React.FC<MediaResponsePreviewProps> = ({ media, title }) => {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [imgZoom, setImgZoom] = useState<'fit' | 'original'>('fit');
  const [imgDimensions, setImgDimensions] = useState<{ width: number; height: number } | null>(null);
  const [copied, setCopied] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const formatBytes = (bytes?: number) => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const handleSave = async () => {
    if (!media.cleanB64 && !media.dataUri) return;
    setIsSaving(true);
    const fileName = `${title || 'response-media'}.${media.extension}`;
    const payload = media.cleanB64 || media.dataUri;
    await saveBase64ToFile(payload, fileName, media.mimeType);
    setIsSaving(false);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(media.dataUri);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const getMediaBadge = () => {
    switch (media.previewType) {
      case 'image':
        return { label: `IMAGE (${media.extension.toUpperCase()})`, icon: 'pic_line', color: 'text-amber-400 bg-amber-500/10 border-amber-500/30' };
      case 'svg':
        return { label: 'SVG VECTOR', icon: 'paint_brush_line', color: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/30' };
      case 'audio':
        return { label: `AUDIO (${media.extension.toUpperCase()})`, icon: 'music_2_line', color: 'text-purple-400 bg-purple-500/10 border-purple-500/30' };
      case 'video':
        return { label: `VIDEO (${media.extension.toUpperCase()})`, icon: 'video_line', color: 'text-sky-400 bg-sky-500/10 border-sky-500/30' };
      case 'pdf':
        return { label: 'PDF DOCUMENT', icon: 'file_line', color: 'text-rose-400 bg-rose-500/10 border-rose-500/30' };
    }
  };

  const badge = getMediaBadge();

  return (
    <div className="h-full flex flex-col bg-background/50 rounded-lg border border-border overflow-hidden select-none">
      {/* Top Media Toolbar */}
      <div className="p-2 bg-header border-b border-border flex items-center justify-between gap-2 font-mono text-xs shrink-0">
        <div className="flex items-center gap-2 overflow-hidden">
          <span className={`px-2 py-0.5 rounded font-bold text-3xs border flex items-center gap-1 ${badge.color}`}>
            <MingCuteIcon name={badge.icon} size={12} />
            <span>{badge.label}</span>
          </span>
          <span className="text-muted-foreground text-2xs truncate">{media.mimeType}</span>
          {media.sizeBytes ? (
            <span className="text-muted-foreground text-2xs">• {formatBytes(media.sizeBytes)}</span>
          ) : null}
          {imgDimensions && (
            <span className="text-muted-foreground text-2xs">• {imgDimensions.width}×{imgDimensions.height}px</span>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {(media.previewType === 'image' || media.previewType === 'svg') && (
            <button
              type="button"
              onClick={() => setImgZoom(imgZoom === 'fit' ? 'original' : 'fit')}
              className="px-2 py-0.5 rounded bg-surface hover:bg-neutral-subtle border border-border text-foreground text-2xs font-sans font-medium cursor-pointer transition-colors"
              title="Toggle Fit vs Actual Size"
            >
              {imgZoom === 'fit' ? '100% Size' : 'Fit to Window'}
            </button>
          )}

          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-surface hover:bg-neutral-subtle border border-border text-foreground text-2xs font-sans font-medium cursor-pointer transition-colors"
            title="Copy Data URI"
          >
            <MingCuteIcon name={copied ? 'check_line' : 'copy_2_line'} size={12} />
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>

          <button
            type="button"
            onClick={handleSave}
            disabled={isSaving}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/30 text-emerald-400 text-2xs font-sans font-semibold cursor-pointer transition-colors"
            title="Save Decoded File to Disk"
          >
            <MingCuteIcon name="download_line" size={12} />
            <span>{isSaving ? 'Saving...' : 'Save File'}</span>
          </button>

          <button
            type="button"
            onClick={() => setIsModalOpen(true)}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-primary/10 hover:bg-primary/20 border border-primary/30 text-primary text-2xs font-sans font-semibold cursor-pointer transition-colors"
            title="Open Fullscreen Preview Modal"
          >
            <MingCuteIcon name="fullscreen_line" size={12} />
            <span>Expand</span>
          </button>
        </div>
      </div>

      {/* Media Content Stage */}
      <div className="flex-1 overflow-auto flex items-center justify-center p-4 min-h-0 bg-surface/30">
        {media.previewType === 'image' && (
          <div className="relative max-w-full max-h-full flex items-center justify-center p-2 rounded border border-border/60 bg-[radial-gradient(#ffffff15_1px,transparent_1px)] [background-size:12px_12px]">
            <img
              src={media.dataUri}
              alt="Response Preview"
              onLoad={(e) => {
                const img = e.currentTarget;
                setImgDimensions({ width: img.naturalWidth, height: img.naturalHeight });
              }}
              className={`rounded shadow-lg transition-all ${
                imgZoom === 'fit' ? 'max-h-[75vh] max-w-full object-contain' : 'max-w-none'
              }`}
            />
          </div>
        )}

        {media.previewType === 'svg' && (
          <div className="relative max-w-full max-h-full flex items-center justify-center p-4 rounded border border-border/60 bg-surface">
            <img
              src={media.dataUri}
              alt="SVG Preview"
              className={imgZoom === 'fit' ? 'max-h-[75vh] max-w-full object-contain' : 'max-w-none'}
            />
          </div>
        )}

        {media.previewType === 'audio' && (
          <div className="w-full max-w-lg p-6 bg-surface border border-border rounded-xl shadow-lg flex flex-col items-center gap-4 text-center">
            <div className="w-16 h-16 rounded-full bg-purple-500/10 border border-purple-500/30 flex items-center justify-center text-purple-400">
              <MingCuteIcon name="music_2_line" size={32} />
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="font-semibold text-foreground text-sm font-sans">{title || 'Audio Stream'}</span>
              <span className="text-xs text-muted-foreground font-mono">{media.mimeType}</span>
            </div>
            <audio controls src={media.dataUri} className="w-full mt-2" />
          </div>
        )}

        {media.previewType === 'video' && (
          <div className="w-full max-w-3xl max-h-full flex flex-col items-center justify-center bg-black/40 rounded-xl overflow-hidden border border-border p-2">
            <video
              controls
              src={media.dataUri}
              className="max-h-[70vh] max-w-full rounded shadow-xl"
            />
          </div>
        )}

        {media.previewType === 'pdf' && (
          <div className="w-full h-full flex flex-col bg-background rounded-lg border border-border overflow-hidden">
            <div className="p-2 bg-header border-b border-border flex items-center justify-between text-xs">
              <span className="font-semibold text-foreground flex items-center gap-1.5">
                <MingCuteIcon name="file_line" size={14} className="text-rose-400" />
                <span>PDF Document Preview</span>
              </span>
              <button
                type="button"
                onClick={() => setIsModalOpen(true)}
                className="px-2 py-0.5 rounded bg-primary text-primary-foreground font-semibold text-xs cursor-pointer"
              >
                Open Fullscreen
              </button>
            </div>
            <iframe
              src={media.dataUri}
              title="PDF Preview"
              className="w-full flex-1 border-none bg-white"
            />
          </div>
        )}
      </div>

      {/* Modal Backdrop */}
      {isModalOpen && (
        <Base64PreviewModal
          isOpen={isModalOpen}
          onClose={() => setIsModalOpen(false)}
          data={media.cleanB64 || media.dataUri}
          fieldName={title || 'response_media'}
        />
      )}
    </div>
  );
};
