import React, { useState, useMemo, useCallback, useRef } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { serializeVisible, deepSearch, selectTextOf } from './json-tree/jsonTreeSerializer';
import { JsonTreeLeaf, HighlightText } from './json-tree/JsonTreeLeaf';

export { serializeVisible, serializeTruncatedTree, deepSearch, selectTextOf } from './json-tree/jsonTreeSerializer';

interface JsonTreeViewerProps {
  label?: string;
  value: unknown;
  isLast?: boolean;
  path?: string;
  searchTerm?: string;
  filterMode?: boolean;
  forceShow?: boolean;
  // Internal state tracking for copy
  _collapsed?: React.MutableRefObject<Set<string>>;
  _expandedArrays?: React.MutableRefObject<Set<string>>;
}

// ─── Main recursive tree node ─────────────────────────────────────────────────
export const JsonTreeViewer: React.FC<JsonTreeViewerProps> = ({
  label,
  value,
  isLast = true,
  path = 'root',
  searchTerm = '',
  filterMode = false,
  forceShow = false,
  _collapsed,
  _expandedArrays,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isArrayExpanded, setIsArrayExpanded] = useState(false);

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

  // Track collapse/expand state in parent refs for copy
  const handleCollapse = useCallback((next: boolean) => {
    setIsCollapsed(next);
    if (_collapsed) {
      if (next) _collapsed.current.add(path);
      else _collapsed.current.delete(path);
    }
  }, [path, _collapsed]);

  const handleArrayExpand = useCallback(() => {
    setIsArrayExpanded(true);
    if (_expandedArrays) _expandedArrays.current.add(path);
  }, [path, _expandedArrays]);

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
    return (
      <JsonTreeLeaf
        label={label}
        value={value}
        isLast={isLast}
        searchTerm={searchTerm}
        filterMode={filterMode}
        shouldForceShow={shouldForceShow}
        valueMatches={valueMatches}
        labelMatches={labelMatches}
      />
    );
  }

  // Filter check for objects/arrays
  if (filterMode && searchTerm && !shouldForceShow && processedItems.length === 0) {
    return null;
  }

  const isEmpty = processedItems.length === 0;
  const isLongArray = isArray && processedItems.length > 1;
  const effectiveShowAll = isArrayExpanded || !!searchTerm;
  const visibleItems =
    isLongArray && !effectiveShowAll ? processedItems.slice(0, 1) : processedItems;

  return (
    <div className="font-mono text-xs leading-relaxed py-0.5">
      <div className="flex items-start">
        <button
          type="button"
          onClick={() => handleCollapse(!isCollapsed)}
          className="w-4 h-4 shrink-0 flex items-center justify-center cursor-pointer text-muted-foreground hover:text-foreground transition-colors mr-1"
          disabled={isEmpty}
        >
          {!isEmpty && (isCollapsed ? '▶' : '▼')}
        </button>
        <div className="flex-1 min-w-0 flex items-center flex-wrap">
          {label && (
            <span
              className="text-primary mr-1 whitespace-nowrap font-semibold cursor-text"
              onDoubleClick={(e) => {
                e.preventDefault();
                selectTextOf(e.currentTarget);
              }}
            >
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
                onClick={() => handleCollapse(false)}
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
                  _collapsed={_collapsed}
                  _expandedArrays={_expandedArrays}
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
                  _collapsed={_collapsed}
                  _expandedArrays={_expandedArrays}
                />
              ))}

          {isLongArray && !effectiveShowAll && (
            <div
              onClick={handleArrayExpand}
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

// ─── Root wrapper with copy button ────────────────────────────────────────────
interface JsonTreeViewerRootProps {
  value: unknown;
  searchTerm?: string;
  filterMode?: boolean;
}

export const JsonTreeViewerRoot: React.FC<JsonTreeViewerRootProps> = ({
  value,
  searchTerm,
  filterMode,
}) => {
  const collapsedRef = useRef<Set<string>>(new Set());
  const expandedArraysRef = useRef<Set<string>>(new Set());
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    const text = serializeVisible(value, collapsedRef.current, expandedArraysRef.current, 'root', 0);
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [value]);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={handleCopy}
        className={`absolute top-0 right-0 z-10 flex items-center gap-1 px-2 py-0.5 rounded text-[10px] border transition-colors cursor-pointer font-sans ${
          copied
            ? 'bg-emerald-500/20 border-emerald-500/40 text-emerald-400'
            : 'bg-surface border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
        }`}
        title="Copy visible JSON"
      >
        <MingCuteIcon name={copied ? 'check_line' : 'copy_2_line'} size={11} />
        <span>{copied ? 'Copied!' : 'Copy'}</span>
      </button>
      <JsonTreeViewer
        value={value}
        searchTerm={searchTerm}
        filterMode={filterMode}
        _collapsed={collapsedRef}
        _expandedArrays={expandedArraysRef}
      />
    </div>
  );
};
