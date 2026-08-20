import { useState, useMemo, useEffect } from 'react';
import { Modal } from '../ui/Modal';
import { detectBase64, saveBase64ToFile } from '@/lib/utils/base64Helper';
import { useNotification } from '../ui/NotificationProvider';
import JsonViewer from '../ui/JsonViewer';

interface Base64PreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  data: string;
  fieldName?: string;
}

export function Base64PreviewModal({
  isOpen,
  onClose,
  data,
  fieldName = 'field',
}: Base64PreviewModalProps) {
  const { notify } = useNotification();
  const [activeTab, setActiveTab] = useState<'preview' | 'raw'>('preview');
  const [isSaving, setIsSaving] = useState(false);
  const [overrideMode, setOverrideMode] = useState<'auto' | 'pdf' | 'image' | 'audio' | 'video' | 'text' | 'json' | 'hex'>('auto');

  const info = useMemo(() => detectBase64(data), [data]);

  const effectiveType = useMemo(() => {
    if (overrideMode !== 'auto') return overrideMode;
    return info?.previewType || 'binary';
  }, [overrideMode, info]);

  const effectiveMimeType = useMemo(() => {
    if (overrideMode === 'pdf') return 'application/pdf';
    if (overrideMode === 'image') return 'image/png';
    if (overrideMode === 'audio') return 'audio/mp3';
    if (overrideMode === 'video') return 'video/mp4';
    if (overrideMode === 'json') return 'application/json';
    if (overrideMode === 'text') return 'text/plain';
    return info?.mimeType || 'application/octet-stream';
  }, [overrideMode, info]);

  const effectiveExt = useMemo(() => {
    if (overrideMode === 'pdf') return 'pdf';
    if (overrideMode === 'image') return 'png';
    if (overrideMode === 'audio') return 'mp3';
    if (overrideMode === 'video') return 'mp4';
    if (overrideMode === 'json') return 'json';
    if (overrideMode === 'text') return 'txt';
    return info?.extension || 'bin';
  }, [overrideMode, info]);

  const decodedText = useMemo(() => {
    if (!info) return '';
    try {
      const binaryString = atob(info.cleanB64);
      return binaryString;
    } catch {
      return '';
    }
  }, [info]);

  const decodedJson = useMemo(() => {
    if ((effectiveType === 'json' || info?.previewType === 'json') && decodedText) {
      try {
        return JSON.parse(decodedText);
      } catch {
        return null;
      }
    }
    return null;
  }, [effectiveType, info, decodedText]);

  const dataUri = useMemo(() => {
    if (!info) return '';
    return `data:${effectiveMimeType};base64,${info.cleanB64}`;
  }, [info, effectiveMimeType]);

  const hexLines = useMemo(() => {
    if (!decodedText || effectiveType === 'image' || effectiveType === 'pdf') return [];
    const maxBytes = Math.min(decodedText.length, 4096);
    const lines: { offset: string; hex: string; ascii: string }[] = [];
    for (let i = 0; i < maxBytes; i += 16) {
      const chunk = decodedText.slice(i, i + 16);
      const offset = i.toString(16).padStart(6, '0');
      let hex = '';
      let ascii = '';
      for (let j = 0; j < 16; j++) {
        if (j < chunk.length) {
          const code = chunk.charCodeAt(j);
          hex += code.toString(16).padStart(2, '0') + ' ';
          ascii += code >= 32 && code <= 126 ? chunk[j] : '.';
        } else {
          hex += '   ';
        }
      }
      lines.push({ offset, hex, ascii });
    }
    return lines;
  }, [decodedText, effectiveType]);

  useEffect(() => {
    if (isOpen) {
      setActiveTab('preview');
      setOverrideMode('auto');
    }
  }, [isOpen]);

  if (!info) {
    return (
      <Modal isOpen={isOpen} onClose={onClose} title="Invalid Base64 Data" maxWidth="md">
        <div className="p-6 text-center text-zinc-400 text-xs">
          The selected data could not be parsed as valid Base64.
        </div>
      </Modal>
    );
  }

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const defaultFileName = `${fieldName.replace(/[^a-zA-Z0-9_-]/g, '_')}.${effectiveExt}`;

  const handleSave = async () => {
    setIsSaving(true);
    try {
      const res = await saveBase64ToFile(info.cleanB64, defaultFileName, effectiveMimeType);
      if (res.success) {
        notify.success(`File saved successfully${res.path ? `: ${res.path}` : ''}`);
      }
    } catch (e) {
      notify.error(`Failed to save file: ${e}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleCopyRaw = () => {
    navigator.clipboard.writeText(info.cleanB64);
    notify.success('Base64 string copied to clipboard');
  };

  const handleCopyDecoded = () => {
    if (decodedText) {
      navigator.clipboard.writeText(decodedText);
      notify.success('Decoded content copied to clipboard');
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Base64 Viewer - ${fieldName}`}
      maxWidth="3xl"
      footer={
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-[10px] font-mono text-zinc-400">
            <span className="bg-zinc-800 px-2 py-0.5 rounded text-sky-400 uppercase font-bold">{effectiveMimeType}</span>
            <span className="bg-zinc-800 px-2 py-0.5 rounded text-emerald-400 font-bold">{formatSize(info.sizeBytes)}</span>
          </div>

          <div className="flex items-center gap-2">
            {(effectiveType === 'text' || effectiveType === 'json') && (
              <button
                onClick={handleCopyDecoded}
                className="px-3 py-1 text-xs font-bold text-zinc-300 hover:text-white bg-zinc-900 border border-zinc-800 rounded hover:border-zinc-700 transition-colors"
              >
                Copy Decoded
              </button>
            )}

            <button
              onClick={handleCopyRaw}
              className="px-3 py-1 text-xs font-bold text-zinc-300 hover:text-white bg-zinc-900 border border-zinc-800 rounded hover:border-zinc-700 transition-colors"
            >
              Copy Base64
            </button>

            <button
              onClick={handleSave}
              disabled={isSaving}
              className="px-4 py-1 text-xs font-bold text-zinc-950 bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 rounded transition-colors flex items-center gap-1.5 cursor-pointer"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="7 10 12 15 17 10"></polyline>
                <line x1="12" y1="15" x2="12" y2="3"></line>
              </svg>
              <span>{isSaving ? 'Saving...' : `Save as .${effectiveExt}`}</span>
            </button>
          </div>
        </div>
      }
    >
      <div className="flex flex-col h-[520px]">
        {/* Tab & Open As controls */}
        <div className="flex items-center justify-between px-4 py-2 bg-zinc-900/50 border-b border-zinc-800 shrink-0 gap-3">
          <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
            <button
              onClick={() => setActiveTab('preview')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${
                activeTab === 'preview' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Preview ({effectiveType})
            </button>
            <button
              onClick={() => setActiveTab('raw')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-widest rounded transition-all ${
                activeTab === 'raw' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Raw Base64
            </button>
          </div>

          {/* OPEN AS DROPDOWN */}
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-bold text-zinc-400 uppercase tracking-widest">Open As:</span>
            <select
              value={overrideMode}
              onChange={(e) => {
                setOverrideMode(e.target.value as any);
                setActiveTab('preview');
              }}
              className="bg-zinc-950 border border-zinc-800 rounded px-2.5 py-1 text-[10px] text-amber-400 font-bold focus:outline-none focus:border-amber-500 cursor-pointer"
            >
              <option value="auto">⚡ Auto Detect ({info.previewType.toUpperCase()})</option>
              <option value="pdf">📄 PDF Document (.pdf)</option>
              <option value="image">🖼️ Image (.png / .jpg)</option>
              <option value="audio">🔊 Audio (.mp3 / .wav)</option>
              <option value="video">🎬 Video (.mp4)</option>
              <option value="text">📝 Text / Code (.txt)</option>
              <option value="json">CODE JSON (.json)</option>
              <option value="hex">🔢 Hex Dump</option>
            </select>
          </div>
        </div>

        {/* Content Panel */}
        <div className="flex-1 overflow-auto p-4 bg-zinc-950/70">
          {activeTab === 'raw' ? (
            <pre className="text-xs font-mono text-zinc-300 whitespace-pre-wrap break-all select-all">
              {info.cleanB64}
            </pre>
          ) : effectiveType === 'image' ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <img
                src={dataUri}
                alt="Base64 Image Preview"
                className="max-w-full max-h-[400px] object-contain rounded border border-zinc-800 shadow-2xl bg-zinc-900/40 p-2"
              />
            </div>
          ) : effectiveType === 'pdf' ? (
            <iframe
              src={dataUri}
              className="w-full h-full rounded border border-zinc-800 bg-white min-h-[440px]"
              title="PDF Preview"
            />
          ) : effectiveType === 'audio' ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <audio controls src={dataUri} className="w-full max-w-md" />
            </div>
          ) : effectiveType === 'video' ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <video controls src={dataUri} className="max-w-full max-h-[400px] rounded border border-zinc-800" />
            </div>
          ) : effectiveType === 'json' && decodedJson ? (
            <JsonViewer value={decodedJson} path="base64-json-root" />
          ) : effectiveType === 'text' ? (
            <pre className="text-xs font-mono text-emerald-400 whitespace-pre-wrap break-all select-all">
              {decodedText || '[No readable text content]'}
            </pre>
          ) : (
            /* Hex Dump View for binary data */
            <div className="font-mono text-xs text-zinc-300 space-y-0.5 select-text">
              <div className="text-[10px] text-zinc-500 border-b border-zinc-800 pb-1 mb-2 font-bold flex gap-4">
                <span className="w-16">Offset</span>
                <span className="flex-1">Hex</span>
                <span className="w-36">ASCII</span>
              </div>
              {hexLines.map((line, idx) => (
                <div key={idx} className="flex gap-4 hover:bg-zinc-900/50 px-1 rounded">
                  <span className="w-16 text-zinc-600 font-bold">{line.offset}</span>
                  <span className="flex-1 text-sky-400 tracking-wider font-mono">{line.hex}</span>
                  <span className="w-36 text-emerald-400 tracking-widest">{line.ascii}</span>
                </div>
              ))}
              {decodedText.length > 4096 && (
                <div className="text-[10px] text-zinc-500 italic pt-2">
                  Showing first 4,096 bytes of {formatSize(info.sizeBytes)}. Use &quot;Save as File&quot; to download the complete payload.
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
