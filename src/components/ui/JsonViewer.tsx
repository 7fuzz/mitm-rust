import { useEffect, useMemo } from "react";

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

export default function JsonViewer({
  label, value, isLast = true, path,
  searchTerm = "", filterMode = false, forceShow = false,
  redactedKeys = [],
  onToggleRedact = undefined,
  collapsedPaths = new Set<string>(),
  onToggleCollapse = undefined,
  expandedArrays = new Set<string>(),
  onExpandArray = undefined
}: JsonViewerProps) {

  // Use centralized collapse state if provided, otherwise default to expanded
  const isCollapsed = collapsedPaths.has(path);
  const expanded = !isCollapsed;

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

    return (
      <div className="font-mono text-[13px] leading-relaxed flex items-center group" data-path={path}>
        <div className="w-6 shrink-0 flex justify-center">
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
        <div className="flex-1 min-w-0 break-all flex items-center">
          {label && (
            <span className="text-sky-text mr-1 whitespace-nowrap">
              &quot;<HighlightText text={label} query={searchTerm} />&quot;:
            </span>
          )}
          <span className={valueColor}>
            {isString && !isRedacted && '"'}
            <HighlightText text={formattedValue} query={searchTerm} />
            {isString && !isRedacted && '"'}
          </span>
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
  const effectiveShowAll = expandedArrays.has(path) || !!searchTerm;

  const visibleItems = isLongArray && !effectiveShowAll ? processedItems.slice(0, 1) : processedItems;

  const handleToggle = () => {
    if (onToggleCollapse) onToggleCollapse(path);
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
            <div className="text-zinc-500 hover:text-sky-text text-xs py-1 cursor-pointer select-none pl-2 flex items-center gap-1" onClick={() => onExpandArray && onExpandArray(path)}>
              <span className="bg-zinc-800 px-1.5 py-0.5 rounded">+{processedItems.length - 1} more items</span>
            </div>
          )}
        </div>
      )}
      {expanded && !isEmpty && <div className="ml-6 text-zinc-400">{closeBracket}{!isLast ? "," : ""}</div>}
    </div>
  );
}
