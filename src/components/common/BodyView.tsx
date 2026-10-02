import React, { useEffect, useMemo, useState } from 'react';
import { CodeEditor } from './CodeEditor';
import { HexViewer } from './HexViewer';
import { JsonTreeViewerRoot } from './JsonTreeViewer';
import { MediaResponsePreview } from './MediaResponsePreview';
import { detectMediaResponse, type MediaDetectionResult } from '../../utils/mediaDetector';
import { isJsonString } from '../../utils/bodyConverters';

type BodyHeaders = Array<{ key: string; value: string }>;

/**
 * View formats that fit a body (Preview for media, Tree for JSON, plus Pretty/Raw/Hex/HTML)
 * and the selected one, which falls back when the body changes and no longer fits it.
 */
export function useBodyFormat(body: string, headers: BodyHeaders) {
  const [format, setFormat] = useState<string>('pretty');

  const mediaInfo = useMemo(() => detectMediaResponse(body, headers), [body, headers]);

  const isJson = useMemo(() => {
    if (!body) return false;
    const hasJsonHeader = headers.some(
      (h) => h.key?.toLowerCase() === 'content-type' && h.value?.toLowerCase().includes('json')
    );
    return hasJsonHeader || isJsonString(body);
  }, [body, headers]);

  // Auto-switch to preview when media is detected
  useEffect(() => {
    if (mediaInfo) {
      setFormat('preview');
    } else if ((format === 'tree' && !isJson) || format === 'preview') {
      setFormat('pretty');
    }
  }, [mediaInfo, isJson, body]);

  const options = useMemo(() => {
    const list: Array<{ value: string; label: string }> = [];
    if (mediaInfo) {
      list.push({ value: 'preview', label: `Preview (${mediaInfo.previewType.toUpperCase()})` });
    }
    list.push({ value: 'pretty', label: 'Pretty' });
    if (isJson) {
      list.push({ value: 'tree', label: 'Tree' });
    }
    list.push(
      { value: 'raw', label: 'Raw' },
      { value: 'hex', label: 'Hex' },
      { value: 'html', label: 'HTML' }
    );
    return list;
  }, [mediaInfo, isJson]);

  return { format, setFormat, options, mediaInfo };
}

interface BodyViewProps {
  body: string;
  format: string;
  mediaInfo: MediaDetectionResult | null;
  /** Title for the media preview element */
  previewTitle: string;
  emptyMessage?: string;
}

/** Renders a body in the format picked via `useBodyFormat`. */
export const BodyView: React.FC<BodyViewProps> = ({ body, format, mediaInfo, previewTitle, emptyMessage = 'No body content' }) => {
  const [treeSearch, setTreeSearch] = useState('');
  const [treeFilterMode, setTreeFilterMode] = useState(false);

  if (format === 'preview' && mediaInfo) {
    return <MediaResponsePreview media={mediaInfo} title={previewTitle} />;
  }

  if (!body) {
    return <div className="h-full flex items-center justify-center text-muted-foreground italic text-xs">{emptyMessage}</div>;
  }

  if (format === 'tree') {
    try {
      const parsed = JSON.parse(body);
      return (
        <div className="h-full flex flex-col overflow-hidden">
          <div className="flex items-center gap-2 font-mono text-xs px-2 py-1.5 border-b border-border">
            <input
              type="text"
              placeholder="Search JSON tree..."
              value={treeSearch}
              onChange={(e) => setTreeSearch(e.target.value)}
              className="bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary w-48"
            />
            <button
              type="button"
              onClick={() => setTreeFilterMode(!treeFilterMode)}
              className={`px-2 py-0.5 text-3xs font-bold rounded border cursor-pointer ${
                treeFilterMode
                  ? 'bg-amber-500/20 text-amber-400 border-amber-500/40'
                  : 'bg-background text-muted-foreground border-border hover:text-foreground'
              }`}
              title="Filter mode: show matching nodes only"
            >
              {treeFilterMode ? 'Filter On' : 'Filter Off'}
            </button>
          </div>
          <div className="flex-1 overflow-auto p-2 font-mono">
            <JsonTreeViewerRoot value={parsed} searchTerm={treeSearch} filterMode={treeFilterMode} />
          </div>
        </div>
      );
    } catch {
      return <CodeEditor value={body} language="plaintext" readOnly bare />;
    }
  }

  if (format === 'pretty') {
    try {
      return <CodeEditor value={JSON.stringify(JSON.parse(body), null, 2)} language="json" readOnly bare />;
    } catch {
      return <CodeEditor value={body} language={body.trim().startsWith('<') ? 'html' : 'plaintext'} readOnly bare />;
    }
  }

  if (format === 'hex') {
    return <HexViewer content={body} />;
  }

  if (format === 'html') {
    // Empty sandbox: no scripts, no same-origin access to the app or its Tauri bridge
    return <iframe sandbox="" srcDoc={body} title="HTML preview" className="w-full h-full bg-white border-0" />;
  }

  return <CodeEditor value={body} language="plaintext" readOnly bare />;
};
