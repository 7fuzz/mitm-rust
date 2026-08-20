import { useState, useEffect, useMemo } from "react";
import { detectBase64, saveBase64ToFile } from "@/lib/utils/base64Helper";
import { Base64PreviewModal } from "../Modals/Base64PreviewModal";

interface JsonViewerProps {
  label?: string;
  value: unknown;
  isLast?: boolean;
  expandSignal?: boolean;
  collapseSignal?: boolean;
  path: string;
  searchTerm?: string;
  filterMode?: boolean;
  forceShow?: boolean;
  redactedKeys?: string[];
  onToggleRedact?: (key: string) => void;
  collapsedPaths?: Set<string>;
  onToggleCollapse?: (path: string, force?: boolean) => void;
  expandedArrays?: Set<string>;
  onExpandArray?: (path: string) => void;
}

// --- Deep Search Recursion ---
const deepSearch = (obj: unknown, term: string): boolean => {
  if (!term) return false;
  if (obj === null || typeof obj !== 'object') {
    return String(obj).toLowerCase().includes(term);
  }
  if (Array.isArray(obj)) {
    return obj.some(item => deepSearch(item, term));
  }
  return Object.entries(obj as Record<string, unknown>).some(([k, v]) =>
    k.toLowerCase().includes(term) || deepSearch(v, term)
  );
};

// --- Text Highlighter ---
const HighlightText = ({ text, query }: { text: string; query: string }) => {
  if (!query) return <>{text}</>;
  const parts = text.toString().split(new RegExp(`(${query})`, 'gi'));
  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === query.toLowerCase() ? (
          <span key={i} className="bg-amber-500/50 text-zinc-50 rounded-sm px-px">{part}</span>
        ) : (
          part
        )
      )}
    </>
  );
};

// --- Format Size Helper ---
const formatSize = (str: string): string => {
  let bytes = str.length;
  try {
    bytes = new Blob([str]).size;
  } catch {
    // fallback
  }
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

// --- Leaf Node Component for Base64 detection & Actions ---
function Base64Actions({ value, label }: { value: string; label?: string }) {
  const [isOpen, setIsOpen] = useState(false);
  const [showInline, setShowInline] = useState(false);
  const info = useMemo(() => detectBase64(value), [value]);

  if (!info) return null;

  const dataUri = `data:${info.mimeType};base64,${info.cleanB64}`;

  const handleSave = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const name = label ? `${label}.${info.extension}` : `file.${info.extension}`;
    await saveBase64ToFile(info.cleanB64, name, info.mimeType);
  };

  const isMedia = info.previewType === 'image' || info.previewType === 'audio' || info.previewType === 'video' || info.previewType === 'pdf';

  return (
    <div className="inline-flex flex-col gap-1 align-middle">
      <span className="inline-flex items-center gap-1 ml-2 select-none shrink-0">
        <span className="text-[9px] font-bold text-amber-400 bg-amber-500/10 border border-amber-500/30 px-1.5 py-0.5 rounded flex items-center gap-1">
          ⚡ Base64 ({info.extension.toUpperCase()})
        </span>

        {isMedia && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setShowInline(!showInline);
            }}
            className={`text-[10px] font-bold px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1 border ${
              showInline
                ? 'bg-purple-500/20 text-purple-300 border-purple-500/40'
                : 'bg-zinc-800 text-zinc-300 border-zinc-700 hover:text-white'
            }`}
            title="Toggle Inline Live Preview"
          >
            <span>{showInline ? '▼ Hide Live' : '▶ Live Preview'}</span>
          </button>
        )}

        <button
          onClick={(e) => {
            e.stopPropagation();
            setIsOpen(true);
          }}
          className="text-[10px] font-bold text-sky-400 bg-sky-500/10 border border-sky-500/30 hover:bg-sky-500/20 px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1"
          title="Open Full Preview Modal"
        >
          <span>👁️ Open</span>
        </button>

        <button
          onClick={handleSave}
          className="text-[10px] font-bold text-emerald-400 bg-emerald-500/10 border border-emerald-500/30 hover:bg-emerald-500/20 px-1.5 py-0.5 rounded transition-all cursor-pointer flex items-center gap-1"
          title="Save Decoded Base64 as File"
        >
          <span>💾 Save</span>
        </button>
      </span>

      {/* INLINE LIVE PREVIEW */}
      {showInline && isMedia && (
        <div className="my-1.5 ml-2 p-2 bg-zinc-950 border border-zinc-800 rounded-lg shadow-inner flex flex-col gap-2 max-w-md animate-in fade-in duration-150">
          <div className="flex items-center justify-between text-[9px] font-mono text-zinc-500 border-b border-zinc-900 pb-1">
            <span className="font-bold text-zinc-400">Live Preview ({info.mimeType})</span>
            <button
              onClick={() => setIsOpen(true)}
              className="text-sky-400 hover:underline cursor-pointer"
            >
              Full Screen ↗
            </button>
          </div>

          {info.previewType === 'image' && (
            <img
              src={dataUri}
              alt="Live Base64 Preview"
              className="max-h-48 max-w-full object-contain rounded border border-zinc-800 bg-zinc-900/50 p-1 cursor-pointer hover:opacity-90 transition-opacity"
              onClick={() => setIsOpen(true)}
              title="Click for full view"
            />
          )}

          {info.previewType === 'audio' && (
            <audio controls src={dataUri} className="w-full h-8" />
          )}

          {info.previewType === 'video' && (
            <video controls src={dataUri} className="max-h-48 max-w-full rounded border border-zinc-800" />
          )}

          {info.previewType === 'pdf' && (
            <div className="flex items-center gap-3 p-2 bg-zinc-900/50 rounded border border-zinc-800">
              <span className="text-rose-400 font-bold text-xs">📄 PDF Document</span>
              <button
                onClick={() => setIsOpen(true)}
                className="px-2.5 py-1 text-[10px] font-bold text-sky-400 bg-sky-500/10 border border-sky-500/30 rounded hover:bg-sky-500/20"
              >
                Open PDF Viewer
              </button>
            </div>
          )}
        </div>
      )}

      <Base64PreviewModal
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        data={value}
        fieldName={label || 'json_field'}
      />
    </div>
  );
}

export default function JsonViewer({
  label, value, isLast = true, path,
  searchTerm = "", filterMode = false, forceShow = false,
  redactedKeys = [],
  onToggleRedact = undefined,
  collapsedPaths,
  onToggleCollapse,
  expandedArrays,
  onExpandArray
}: JsonViewerProps) {
  // --- INTERNAL STATE FALLBACKS ---
  const [internalCollapsed, setInternalCollapsed] = useState(false);
  const [internalArrayExpanded, setInternalArrayExpanded] = useState(false);
  const [isLongTextExpanded, setIsLongTextExpanded] = useState(false);

  // Use centralized collapse state if provided, otherwise fallback to internal state
  const isCollapsed = onToggleCollapse !== undefined 
    ? (collapsedPaths?.has(path) ?? false)
    : internalCollapsed;
  
  const expanded = !isCollapsed;

  const isArrayExpanded = onExpandArray !== undefined
    ? (expandedArrays?.has(path) ?? false)
    : internalArrayExpanded;

  const termLower = searchTerm.toLowerCase();
  const labelMatches = label ? label.toLowerCase().includes(termLower) : false;
  const shouldForceShow = forceShow || labelMatches;

  const isPrimitive = value === null || typeof value !== "object";
  const valueMatches = isPrimitive ? String(value).toLowerCase().includes(termLower) : false;
  
  // Check filter condition - but don't return early, handle in hook instead
  const shouldFilterOut = filterMode && searchTerm && !shouldForceShow && !valueMatches && !labelMatches && isPrimitive;

  // --- Hooks - must be called unconditionally ---
  const containsMatch = useMemo(() => {
    if (!searchTerm) return false;
    return typeof value === 'object' && value !== null && deepSearch(value, termLower);
  }, [value, searchTerm, termLower]);

  // Handle centralized expanding for search
  useEffect(() => {
    if (searchTerm && containsMatch && isCollapsed && onToggleCollapse) {
      onToggleCollapse(path, false); // Force expand
    }
  }, [searchTerm, containsMatch, isCollapsed, onToggleCollapse, path]);

  const isArray = Array.isArray(value);
  const isObject = value !== null && typeof value === "object" && !isArray;

  // --- Process items for objects/arrays (always call useMemo) ---
  const items = isArray ? value : (isObject ? Object.entries(value) : []);
  const openBracket = isArray ? "[" : "{";
  const closeBracket = isArray ? "]" : "}";

  const processedItems = useMemo(() => {
    if (!filterMode || !searchTerm || shouldForceShow) return items;

    return (items as unknown[]).filter((item: unknown) => {
      if (isArray) {
        if (item === null || typeof item !== 'object') return String(item).toLowerCase().includes(termLower);
        return deepSearch(item, termLower);
      } else {
        const [k, v] = item as [string, unknown];
        if (k.toLowerCase().includes(termLower)) return true;
        if (v === null || typeof v !== 'object') return String(v).toLowerCase().includes(termLower);
        return deepSearch(v, termLower);
      }
    });
  }, [items, searchTerm, filterMode, isArray, termLower, shouldForceShow]);

  // Handle filter-out case for objects/arrays
  const shouldFilterOutObject = filterMode && searchTerm && !shouldForceShow && processedItems.length === 0;

  // --- Primitive Leaf Nodes ---
  if (value === null || typeof value !== "object") {
    let valueColor = "text-zinc-300";
    let formattedValue = String(value);
    const isString = typeof value === "string";

    if (isString) valueColor = "text-emerald-text";
    else if (typeof value === "number") valueColor = "text-amber-400";
    else if (typeof value === "boolean") valueColor = "text-purple-400";
    else if (value === null) { valueColor = "text-rose-400"; formattedValue = "null"; }

    // --- REDACTION LOGIC ---
    const isRedacted = label && redactedKeys.includes(label);
    if (isRedacted) {
      formattedValue = typeof value === 'number' ? '0' : '[REDACTED]';
      valueColor = 'text-rose-400 font-bold bg-rose-500/10 px-1 rounded';
    }

    // Filter check handled via shouldFilterOut
    if (shouldFilterOut) {
      return null;
    }

    // --- LONG TEXT TRUNCATION ---
    const isLongText = isString && !isRedacted && (value as string).length > 200;
    const shouldShowFullText = isLongTextExpanded || (searchTerm && valueMatches);
    const displayedText = isLongText && !shouldShowFullText
      ? (value as string).substring(0, 100)
      : formattedValue;

    return (
      <div className="font-mono text-[13px] leading-relaxed flex items-start group" data-path={path}>
        <div className="w-6 shrink-0 flex justify-center mt-0.5">
          {/* Quick Toggle Lock Icon (Uses SVG Eyes) */}
          {label && onToggleRedact && (
            <button
              onClick={() => onToggleRedact(label)}
              className={`transition-opacity text-[11px] ${isRedacted ? 'text-rose-500 opacity-100' : 'text-zinc-600 opacity-0 group-hover:opacity-100 hover:text-sky-text'}`}
              title={isRedacted ? "Unredact Value" : "Redact Value"}
            >
              {isRedacted ? (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path><line x1="1" y1="1" x2="23" y2="23"></line></svg>
              ) : (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>
              )}
            </button>
          )}
        </div>
        <div className="flex-1 min-w-0 break-all flex items-center flex-wrap">
          {label && (
            <span className="text-sky-text mr-1 whitespace-nowrap">
              &quot;<HighlightText text={label} query={searchTerm} />&quot;:
            </span>
          )}
          <span className={valueColor}>
            {isString && !isRedacted && '"'}
            <HighlightText text={displayedText} query={searchTerm} />
            {isLongText && !shouldShowFullText && '...'}
            {isString && !isRedacted && '"'}
          </span>
          {isLongText && (
            <button
              onClick={() => setIsLongTextExpanded(!shouldShowFullText)}
              className="ml-2 my-0.5 text-[10px] font-sans text-zinc-500 hover:text-zinc-300 bg-zinc-900 border border-zinc-800 hover:border-zinc-700 px-1.5 py-0.5 rounded transition-all select-none whitespace-nowrap flex items-center gap-1 shrink-0"
              title={shouldShowFullText ? "Collapse value" : "Expand value"}
            >
              {shouldShowFullText ? (
                <>
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="18 15 12 9 6 15"></polyline></svg>
                  Collapse
                </>
              ) : (
                <>
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="6 9 12 15 18 9"></polyline></svg>
                  Expand ({formatSize(value as string)})
                </>
              )}
            </button>
          )}
          {isString && !isRedacted && (
            <Base64Actions value={value as string} label={label} />
          )}
          {!isLast && <span className="text-zinc-500">,</span>}
        </div>
      </div>
    );
  }

  // Filter check for objects/arrays (after hooks are called)
  if (shouldFilterOutObject) {
    return null;
  }

  const isEmpty = processedItems.length === 0;
  const isLongArray = isArray && processedItems.length > 1;
  const effectiveShowAll = isArrayExpanded || !!searchTerm;

  const visibleItems = isLongArray && !effectiveShowAll ? processedItems.slice(0, 1) : processedItems;

  const handleToggle = () => {
    if (onToggleCollapse) {
      onToggleCollapse(path);
    } else {
      setInternalCollapsed(!internalCollapsed);
    }
  };

  const handleExpandArray = () => {
    if (onExpandArray) {
      onExpandArray(path);
    } else {
      setInternalArrayExpanded(true);
    }
  };

  return (
    <div className="font-mono text-[13px] leading-relaxed" data-path={path}>
      <div className="flex items-start group">
        <button onClick={handleToggle} className="w-6 shrink-0 flex justify-center items-center cursor-pointer text-zinc-500 hover:text-zinc-300 transition-colors mt-0.5" disabled={isEmpty}>
          {!isEmpty && (expanded ? "▼" : "▶")}
        </button>
        <div className="flex-1 min-w-0 flex items-center flex-wrap">
          {label && (
            <span className="text-sky-text mr-1 whitespace-nowrap">
              &quot;<HighlightText text={label} query={searchTerm} />&quot;:
            </span>
          )}
          <span className="text-zinc-400">{openBracket}</span>
          {isEmpty && <span className="text-zinc-400">{closeBracket}{!isLast ? "," : ""}</span>}
          {!expanded && !isEmpty && (
            <>
              <span className="cursor-pointer text-zinc-500 hover:text-zinc-300 mx-2 bg-zinc-800 px-1 rounded text-[10px]" onClick={handleToggle}>...</span>
              <span className="text-zinc-400">{closeBracket}{!isLast ? "," : ""}</span>
              {isArray && <span className="text-zinc-500 ml-2 text-[11px]">({processedItems.length} items)</span>}
              {isObject && <span className="text-zinc-500 ml-2 text-[11px]">({processedItems.length} keys)</span>}
            </>
          )}
        </div>
      </div>
      {expanded && !isEmpty && (
        <div className="ml-3 pl-3.5 border-l border-zinc-700 hover:border-zinc-500 transition-colors">
          {isArray
            ? (visibleItems as unknown[]).map((item: unknown, index: number) => (
              <JsonViewer key={index} value={item} isLast={index === visibleItems.length - 1} path={`${path}-${index}`} searchTerm={searchTerm} filterMode={filterMode} forceShow={shouldForceShow} redactedKeys={redactedKeys} onToggleRedact={onToggleRedact} collapsedPaths={collapsedPaths} onToggleCollapse={onToggleCollapse} expandedArrays={expandedArrays} onExpandArray={onExpandArray} />
            ))
            : (visibleItems as [string, unknown][]).map(([key, val], index: number) => (
              <JsonViewer key={key} label={key} value={val} isLast={index === visibleItems.length - 1} path={`${path}-${encodeURIComponent(key)}`} searchTerm={searchTerm} filterMode={filterMode} forceShow={shouldForceShow} redactedKeys={redactedKeys} onToggleRedact={onToggleRedact} collapsedPaths={collapsedPaths} onToggleCollapse={onToggleCollapse} expandedArrays={expandedArrays} onExpandArray={onExpandArray} />
            ))}
          {isLongArray && !effectiveShowAll && (
            <div className="text-zinc-500 hover:text-sky-text text-xs py-1 cursor-pointer select-none pl-2 flex items-center gap-1" onClick={handleExpandArray}>
              <span className="bg-zinc-800 px-1.5 py-0.5 rounded">+{processedItems.length - 1} more items</span>
            </div>
          )}
        </div>
      )}
      {expanded && !isEmpty && <div className="ml-6 text-zinc-400">{closeBracket}{!isLast ? "," : ""}</div>}
    </div>
  );
}
