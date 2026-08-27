import React, { useState, useMemo } from 'react';
import { detectBase64, saveBase64ToFile } from '../../utils/base64Helper';
import { Base64PreviewModal } from './Base64PreviewModal';
import { MingCuteIcon } from './MingCuteIcon';

interface JsonTreeViewerProps {
  label?: string;
  value: unknown;
  isLast?: boolean;
  path?: string;
  searchTerm?: string;
  filterMode?: boolean;
  forceShow?: boolean;
}

const deepSearch = (obj: unknown, term: string): boolean => {
  if (!term) return false;
  if (obj === null || typeof obj !== 'object') {
    return String(obj).toLowerCase().includes(term);
  }
  if (Array.isArray(obj)) {
    return obj.some((item) => deepSearch(item, term));
  }
  return Object.entries(obj as Record<string, unknown>).some(
    ([k, v]) => k.toLowerCase().includes(term) || deepSearch(v, term)
  );
};

const HighlightText: React.FC<{ text: string; query?: string }> = ({ text, query }) => {
  if (!query) return <>{text}</>;
  const parts = text.toString().split(new RegExp(`(${query})`, 'gi'));
  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === query.toLowerCase() ? (
          <span key={i} className="bg-amber-500/40 text-amber-200 rounded px-0.5 font-bold">
            {part}
          </span>
        ) : (
          part
        )
      )}
    </>
  );
};

function Base64LeafActions({ value, label }: { value: string; label?: string }) {
  const [isOpen, setIsOpen] = useState(false);
  const [showInline, setShowInline] = useState(false);
  // User directive: Base64 file preview for strings 200+ characters
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
          onClick={handleSave}
          className="font-bold text-emerald-400 bg-emerald-500/10 border border-emerald-500/30 hover:bg-emerald-500/20 px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1"
          title="Save Decoded File"
        >
          <MingCuteIcon name="download_line" size={11} />
          <span>Save</span>
        </button>
      </span>

      {/* Inline Live Media Preview */}
      {showInline && isMedia && (
        <div className="my-1.5 ml-2 p-2 bg-background border border-border rounded-lg shadow-inner flex flex-col gap-2 max-w-md animate-in fade-in duration-150 font-mono text-[10px]">
          <div className="flex items-center justify-between text-muted-foreground border-b border-border pb-1">
            <span className="font-bold text-foreground">Live Preview ({info.mimeType})</span>
            <button onClick={() => setIsOpen(true)} className="text-primary hover:underline cursor-pointer">
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
                onClick={() => setIsOpen(true)}
                className="px-2 py-0.5 font-bold text-primary bg-primary/10 border border-primary/30 rounded"
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
}

export const JsonTreeViewer: React.FC<JsonTreeViewerProps> = ({
  label,
  value,
  isLast = true,
  path = 'root',
  searchTerm = '',
  filterMode = false,
  forceShow = false,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isArrayExpanded, setIsArrayExpanded] = useState(false);
  const [isLongTextExpanded, setIsLongTextExpanded] = useState(false);

  const termLower = searchTerm.toLowerCase();
  const labelMatches = label ? label.toLowerCase().includes(termLower) : false;
  const shouldForceShow = forceShow || labelMatches;

  const isPrimitive = value === null || typeof value !== 'object';
  const valueMatches = isPrimitive ? String(value).toLowerCase().includes(termLower) : false;

  const isArray = Array.isArray(value);
  const isObject = value !== null && typeof value === 'object' && !isArray;

  const items = isArray ? value : isObject ? Object.entries(value) : [];
  const openBracket = isArray ? '[' : '{';
  const closeBracket = isArray ? ']' : '}';

  // Process items for objects & arrays
  const processedItems = useMemo(() => {
    if (!filterMode || !searchTerm || shouldForceShow) return items;

    return (items as unknown[]).filter((item: unknown) => {
      if (isArray) {
        if (item === null || typeof item !== 'object')
          return String(item).toLowerCase().includes(termLower);
        return deepSearch(item, termLower);
      } else {
        const [k, v] = item as [string, unknown];
        if (k.toLowerCase().includes(termLower)) return true;
        if (v === null || typeof v !== 'object')
          return String(v).toLowerCase().includes(termLower);
        return deepSearch(v, termLower);
      }
    });
  }, [items, searchTerm, filterMode, isArray, termLower, shouldForceShow]);

  // Primitive Leaf Node
  if (isPrimitive) {
    if (filterMode && searchTerm && !shouldForceShow && !valueMatches && !labelMatches) {
      return null;
    }

    let valueColor = 'text-foreground';
    let formattedValue = String(value);
    const isString = typeof value === 'string';

    if (isString) valueColor = 'text-emerald-400 font-mono';
    else if (typeof value === 'number') valueColor = 'text-amber-400 font-mono';
    else if (typeof value === 'boolean') valueColor = 'text-purple-400 font-mono';
    else if (value === null) {
      valueColor = 'text-rose-400 font-mono';
      formattedValue = 'null';
    }

    const isLongText = isString && (value as string).length > 200;
    const shouldShowFullText = isLongTextExpanded || (searchTerm && valueMatches);
    const displayedText =
      isLongText && !shouldShowFullText ? (value as string).substring(0, 100) : formattedValue;

    return (
      <div className="font-mono text-xs leading-relaxed flex items-start group py-0.5">
        <div className="flex-1 min-w-0 flex items-center flex-wrap">
          {label && (
            <span className="text-primary mr-1 whitespace-nowrap font-semibold">
              &quot;<HighlightText text={label} query={searchTerm} />&quot;:
            </span>
          )}
          <span className={valueColor}>
            {isString && '"'}
            <HighlightText text={displayedText} query={searchTerm} />
            {isLongText && !shouldShowFullText && '...'}
            {isString && '"'}
          </span>

          {isLongText && (
            <button
              onClick={() => setIsLongTextExpanded(!shouldShowFullText)}
              className="ml-2 text-[10px] font-sans text-muted-foreground hover:text-foreground bg-background border border-border px-1.5 py-0.5 rounded cursor-pointer shrink-0"
            >
              {shouldShowFullText ? 'Collapse' : `Expand (${(value as string).length} chars)`}
            </button>
          )}

          {isString && <Base64LeafActions value={value as string} label={label} />}

          {!isLast && <span className="text-muted-foreground ml-0.5">,</span>}
        </div>
      </div>
    );
  }

  // Filter check for objects/arrays
  if (filterMode && searchTerm && !shouldForceShow && processedItems.length === 0) {
    return null;
  }

  const isEmpty = processedItems.length === 0;
  // User directive: Array truncation (only show 1 item on array with "+ N more items" button)
  const isLongArray = isArray && processedItems.length > 1;
  const effectiveShowAll = isArrayExpanded || !!searchTerm;
  const visibleItems =
    isLongArray && !effectiveShowAll ? processedItems.slice(0, 1) : processedItems;

  return (
    <div className="font-mono text-xs leading-relaxed py-0.5">
      <div className="flex items-start">
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="w-4 h-4 shrink-0 flex items-center justify-center cursor-pointer text-muted-foreground hover:text-foreground transition-colors mr-1"
          disabled={isEmpty}
        >
          {!isEmpty && (isCollapsed ? '▶' : '▼')}
        </button>
        <div className="flex-1 min-w-0 flex items-center flex-wrap">
          {label && (
            <span className="text-primary mr-1 whitespace-nowrap font-semibold">
              &quot;<HighlightText text={label} query={searchTerm} />&quot;:
            </span>
          )}
          <span className="text-muted-foreground font-bold">{openBracket}</span>

          {isEmpty && (
            <span className="text-muted-foreground font-bold">
              {closeBracket}
              {!isLast ? ',' : ''}
            </span>
          )}

          {isCollapsed && !isEmpty && (
            <>
              <span
                onClick={() => setIsCollapsed(false)}
                className="cursor-pointer text-muted-foreground hover:text-foreground mx-1.5 bg-background border border-border px-1.5 py-0.5 rounded text-[10px]"
              >
                ...
              </span>
              <span className="text-muted-foreground font-bold">
                {closeBracket}
                {!isLast ? ',' : ''}
              </span>
              <span className="text-muted-foreground text-[10px] ml-2 italic">
                ({processedItems.length} {isArray ? 'items' : 'keys'})
              </span>
            </>
          )}
        </div>
      </div>

      {!isCollapsed && !isEmpty && (
        <div className="ml-3 pl-3 border-l border-border/80 hover:border-primary/40 transition-colors space-y-0.5">
          {isArray
            ? (visibleItems as unknown[]).map((item: unknown, index: number) => (
                <JsonTreeViewer
                  key={index}
                  value={item}
                  isLast={index === visibleItems.length - 1}
                  path={`${path}-${index}`}
                  searchTerm={searchTerm}
                  filterMode={filterMode}
                  forceShow={shouldForceShow}
                />
              ))
            : (visibleItems as [string, unknown][]).map(([key, val], index: number) => (
                <JsonTreeViewer
                  key={key}
                  label={key}
                  value={val}
                  isLast={index === visibleItems.length - 1}
                  path={`${path}-${encodeURIComponent(key)}`}
                  searchTerm={searchTerm}
                  filterMode={filterMode}
                  forceShow={shouldForceShow}
                />
              ))}

          {/* Show More items button for truncated arrays */}
          {isLongArray && !effectiveShowAll && (
            <div
              onClick={() => setIsArrayExpanded(true)}
              className="text-amber-500 hover:text-amber-400 font-semibold text-[11px] py-1 cursor-pointer select-none pl-2 flex items-center gap-1.5"
            >
              <span className="bg-amber-500/10 border border-amber-500/30 px-2 py-0.5 rounded hover:bg-amber-500/20">
                + {processedItems.length - 1} more items... (click to expand)
              </span>
            </div>
          )}
        </div>
      )}

      {!isCollapsed && !isEmpty && (
        <div className="ml-5 text-muted-foreground font-bold">
          {closeBracket}
          {!isLast ? ',' : ''}
        </div>
      )}
    </div>
  );
};
