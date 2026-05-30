import { useState, useMemo } from 'react';
import { JsonEditor } from '../Editor/JsonEditor';
import JsonViewer from '../ui/JsonViewer';
import { useNotification } from '../ui/NotificationProvider';
import { useTraffic } from '@/hooks/traffic';

export function JsonTool({ splitMode }: { splitMode: 'horizontal' | 'vertical' }) {
  const { notify } = useNotification();

  const { toolkitJson: rawJson, setToolkitJson: setRawJson } = useTraffic();
  const [leftTab, setLeftTab] = useState<'raw' | 'editor'>('raw');

  const [searchTerm, setSearchTerm] = useState('');
  const [filterMode, setFilterMode] = useState(false);
  const [isRedactionActive, setIsRedactionActive] = useState(false);
  const [redactedKeys, setRedactedKeys] = useState<string[]>([]);
  const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(new Set());
  const [expandedArrays, setExpandedArrays] = useState<Set<string>>(new Set());

  const parsedData = useMemo(() => {
    try {
      if (!rawJson || rawJson.trim() === '') return { data: {}, error: null };
      return { data: JSON.parse(rawJson), error: null };
    } catch (_e) {
      return { data: null, error: 'Invalid JSON syntax' };
    }
  }, [rawJson]);

  const handleFormat = () => {
    try {
      setRawJson(JSON.stringify(JSON.parse(rawJson), null, 2));
      notify.success('JSON Formatted');
    } catch {
      notify.error('Invalid JSON');
    }
  };

  const toggleRedactionForNode = (key: string) => {
    setRedactedKeys(prev => prev.includes(key) ? prev.filter(k => k !== key) : [...prev, key]);
  };

  const toggleCollapse = (path: string, forceExpand?: boolean) => {
    setCollapsedPaths(prev => {
      const next = new Set(prev);
      if (forceExpand === false) next.delete(path);
      else if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const expandArray = (path: string) => {
    setExpandedArrays(prev => new Set(prev).add(path));
  };

  // --- NEW REDACTION TOOLS ---

  const clearRedaction = () => {
    setRedactedKeys([]);
    notify.info('Cleared all redactions');
  };

  const redactAll = () => {
    if (parsedData.error) return notify.error('Cannot read tree: ' + parsedData.error);
    const allKeys = new Set<string>(redactedKeys);

    const extractAllKeys = (obj: unknown) => {
      if (Array.isArray(obj)) {
        obj.forEach(extractAllKeys);
      } else if (obj !== null && typeof obj === 'object') {
        const rec = obj as Record<string, unknown>;
        for (const k of Object.keys(rec)) {
          allKeys.add(k);
          extractAllKeys(rec[k]);
        }
      }
    };

    extractAllKeys(parsedData.data);
    setRedactedKeys(Array.from(allKeys));
    setIsRedactionActive(true);
    notify.success('Redacted all values');
  };

  const autoRedact = () => {
    if (parsedData.error) return notify.error('Cannot read tree: ' + parsedData.error);
    const newRedacted = new Set<string>(redactedKeys);

    const isSensitive = (key: string) => {
      const lower = key.toLowerCase();

      // Strict matches for short words to avoid false positives (e.g. 'id' matching 'width')
      const strictMatch = ['id', 'key'];
      for (const kw of strictMatch) {
        if (lower === kw || lower.endsWith(`_${kw}`) || lower.startsWith(`${kw}_`) || lower.endsWith(kw)) {
          return true;
        }
      }

      // Broad matches for obvious secrets
      const broadMatch = ['email', 'phone', 'name', 'token', 'password', 'secret', 'auth', 'hash', 'credential', 'company', 'employee'];
      for (const kw of broadMatch) {
        if (lower.includes(kw)) return true;
      }

      return false;
    };

    const findKeys = (obj: unknown) => {
      if (Array.isArray(obj)) {
        obj.forEach(findKeys);
      } else if (obj !== null && typeof obj === 'object') {
        const rec = obj as Record<string, unknown>;
        for (const k of Object.keys(rec)) {
          if (isSensitive(k)) newRedacted.add(k);
          findKeys(rec[k]);
        }
      }
    };

    findKeys(parsedData.data);
    setRedactedKeys(Array.from(newRedacted));
    setIsRedactionActive(true);
    notify.success('Auto-redacted sensitive fields');
  };

  // --- RECURSIVE DATA EXPORTER ---
  const copyToClipboard = (onlyVisible: boolean) => {
    if (parsedData.error) return notify.error('Cannot copy: ' + parsedData.error);

    const buildExport = (data: unknown, path: string = "root"): unknown => {
      if (onlyVisible && collapsedPaths.has(path)) {
        return Array.isArray(data) ? '[Hidden Array]' : '[Hidden Object]';
      }

      if (Array.isArray(data)) {
        if (onlyVisible && data.length > 1 && !expandedArrays.has(path)) {
          return [
            buildExport(data[0], `${path}-0`),
            `[... ${data.length - 1} more items hidden]`
          ];
        }
        return data.map((item, i) => buildExport(item, `${path}-${i}`));
      }

      if (data !== null && typeof data === 'object') {
        const result: Record<string, unknown> = {};
        const rec = data as Record<string, unknown>;
        for (const [k, v] of Object.entries(rec)) {
          if (isRedactionActive && redactedKeys.includes(k)) {
            result[k] = typeof v === 'number' ? 0 : '[REDACTED]';
          } else {
            result[k] = buildExport(v, `${path}-${encodeURIComponent(k)}`);
          }
        }
        return result;
      }

      return data;
    };

    const finalData = buildExport(parsedData.data);
    navigator.clipboard.writeText(JSON.stringify(finalData, null, 2));
    notify.success(`Copied ${onlyVisible ? 'Visible State' : 'All Data'}`);
  };

  return (
    <div className="flex flex-col h-full space-y-4">

      {/* Top Toolbar */}
      <div className="flex items-center justify-between border-b border-zinc-800 pb-4">

        <div className="flex items-center gap-4">
          <div className="text-[12px] font-black tracking-[0.2em] text-zinc-300 uppercase">
            Dual-Pane Toolkit
          </div>
        </div>

        <div className="flex gap-2">
          <button onClick={handleFormat} className="px-4 py-1.5 bg-zinc-900 text-emerald-text hover:bg-zinc-800 border border-zinc-800 text-[10px] font-bold uppercase rounded transition-colors" title="Format raw code">
            Format Raw
          </button>
          <div className="w-px h-6 bg-zinc-800 mx-2 self-center"></div>
          <button onClick={() => copyToClipboard(true)} className="px-4 py-1.5 bg-zinc-900 text-zinc-300 hover:bg-zinc-800 border border-zinc-800 text-[10px] font-bold uppercase rounded transition-colors" title="Copies only expanded nodes">
            Copy Visible
          </button>
          <button onClick={() => copyToClipboard(false)} className="px-4 py-1.5 bg-sky-900/30 text-sky-text hover:bg-sky-600 hover:text-zinc-950 border border-sky-800 text-[10px] font-bold uppercase rounded transition-colors" title="Copies entire JSON structure">
            Copy All
          </button>
        </div>
      </div>

      {/* 2-Panel Split Engine */}
      <div className={`flex-1 grid gap-6 ${splitMode === 'horizontal' ? 'grid-cols-2' : 'grid-cols-1'} min-h-[600px]`}>

        {/* LEFT PANEL: Editor Engine */}
        <div className="bg-zinc-900/20 border border-zinc-800/50 rounded-xl overflow-hidden flex flex-col shadow-sm">
          <div className="bg-zinc-950 border-b border-zinc-800 p-2 px-4 flex justify-between items-center shrink-0">
            <div className="flex bg-zinc-900 p-1 rounded border border-zinc-800">
              <button onClick={() => setLeftTab('raw')} className={`px-4 py-1 text-[9px] font-black tracking-widest uppercase rounded transition-all ${leftTab === 'raw' ? 'bg-zinc-700 text-white' : 'text-zinc-500 hover:text-zinc-300'}`}>
                Raw Code
              </button>
              <button onClick={() => setLeftTab('editor')} className={`px-4 py-1 text-[9px] font-black tracking-widest uppercase rounded transition-all ${leftTab === 'editor' ? 'bg-zinc-700 text-white' : 'text-zinc-500 hover:text-zinc-300'}`}>
                Tree Editor
              </button>
            </div>
          </div>

          {/* FIXED: Dynamic overflow handling and min-h-0 prevents the scroll fight */}
          <div className={`flex-1 flex flex-col min-h-0 ${leftTab === 'raw' ? 'overflow-hidden' : 'overflow-y-auto'}`}>
            {leftTab === 'raw' ? (
              <textarea
                value={rawJson}
                onChange={(e) => setRawJson(e.target.value)}
                // Removed h-full, added flex-1 so it securely anchors to the wrapper
                className="flex-1 w-full bg-transparent text-emerald-text font-mono text-xs p-4 outline-none resize-none"
                spellCheck={false}
              />
            ) : (
              <JsonEditor initialBody={rawJson} onChange={setRawJson} />
            )}
          </div>
        </div>

        {/* RIGHT PANEL: The Viewer */}
        <div className="bg-zinc-900/20 border border-zinc-800/50 rounded-xl overflow-hidden flex flex-col shadow-sm">

          <div className="bg-zinc-950 border-b border-zinc-800 p-2 px-4 flex flex-col xl:flex-row xl:items-center justify-between gap-3 shrink-0">

            {/* Viewer Controls: Search & Filter */}
            <div className="flex items-center gap-2 flex-1">
              <div className="flex items-center bg-zinc-900 border border-zinc-700 px-2 rounded w-full max-w-xs focus-within:border-sky-500 transition-colors">
                <span className="text-zinc-500">🔍</span>
                <input
                  placeholder="Search nodes..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="w-full bg-transparent p-1.5 outline-none text-[11px] font-mono text-sky-text"
                />
              </div>
              <button
                onClick={() => setFilterMode(!filterMode)}
                className={`px-3 py-1.5 text-[9px] font-bold uppercase tracking-widest rounded border transition-colors ${filterMode ? 'bg-sky-500/20 text-sky-text border-sky-500/50' : 'bg-zinc-900 text-zinc-500 border-zinc-700 hover:text-zinc-300'}`}
                title="Hide non-matching nodes"
              >
                Filter
              </button>
            </div>

            {/* Viewer Controls: Redaction */}
            <div className="flex items-center gap-2">
              <button
                onClick={() => setIsRedactionActive(!isRedactionActive)}
                className={`px-3 py-1.5 text-[9px] font-bold uppercase tracking-widest rounded border transition-colors flex items-center gap-2 ${isRedactionActive ? 'bg-rose-500/20 text-rose-400 border-rose-500/50' : 'bg-zinc-900 text-zinc-500 border-zinc-700 hover:text-zinc-300'}`}
              >
                Redact {isRedactionActive ? 'ON' : 'OFF'}
              </button>

              {/* Quick Action Button Group */}
              {isRedactionActive && (
                <div className="flex bg-zinc-900 border border-zinc-700 rounded p-1 animate-in fade-in zoom-in-95 duration-200">
                  <button
                    onClick={autoRedact}
                    className="px-2 py-1 hover:bg-zinc-800 text-zinc-400 hover:text-sky-text text-[9px] font-bold uppercase tracking-widest rounded transition-colors"
                    title="Auto-mask sensitive fields"
                  >
                    Auto
                  </button>
                  <div className="w-px h-3 bg-zinc-700 self-center"></div>
                  <button
                    onClick={redactAll}
                    className="px-2 py-1 hover:bg-zinc-800 text-zinc-400 hover:text-rose-400 text-[9px] font-bold uppercase tracking-widest rounded transition-colors"
                    title="Mask everything"
                  >
                    All
                  </button>
                  <div className="w-px h-3 bg-zinc-700 self-center"></div>
                  <button
                    onClick={clearRedaction}
                    className="px-2 py-1 hover:bg-zinc-800 text-zinc-400 hover:text-emerald-text text-[9px] font-bold uppercase tracking-widest rounded transition-colors"
                    title="Clear all masks"
                  >
                    Clear
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* Viewer Render Phase */}
          <div className="p-4 h-full overflow-auto">
            {parsedData.error ? (
              <div className="text-rose-500 text-xs font-mono">Syntax Error: {parsedData.error}. Check Editor.</div>
            ) : (
              <JsonViewer
                value={parsedData.data}
                path="root"
                searchTerm={searchTerm}
                filterMode={filterMode}
                redactedKeys={isRedactionActive ? redactedKeys : []}
                onToggleRedact={isRedactionActive ? toggleRedactionForNode : undefined}
                collapsedPaths={collapsedPaths}
                onToggleCollapse={toggleCollapse}
                expandedArrays={expandedArrays}
                onExpandArray={expandArray}
              />
            )}
          </div>
        </div>

      </div>
    </div>
  );
}
