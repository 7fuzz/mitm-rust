import React, { useMemo } from 'react';
import { CodeEditor } from '../../common/CodeEditor';
import { DiffCodeEditor } from '../../common/DiffCodeEditor';
import { SegmentedControl, Select, Switch } from '../../common/ui';
import {
  useUtilitiesStore,
  JsonIndent,
  JsonMode,
  JsonOperation,
} from '../../../stores/useUtilitiesStore';
import { ErrorStrip, PaneAction, StatusPill, ToolPane, readClipboard, textStats, useCopyFeedback } from './ToolLayout';
import {
  ChangeKind,
  JsonChange,
  SAMPLE_DIFF,
  SAMPLE_JSON,
  describeJson,
  diffJson,
  formatJson,
  parseJson,
  prettify,
  previewValue,
} from './json';

const MODE_OPTIONS: { value: JsonMode; label: string }[] = [
  { value: 'format', label: 'Format' },
  { value: 'diff', label: 'Diff' },
];

const OPERATION_OPTIONS: { value: JsonOperation; label: string }[] = [
  { value: 'pretty', label: 'Pretty' },
  { value: 'minify', label: 'Minify' },
  { value: 'escape', label: 'Escape' },
  { value: 'unescape', label: 'Unescape' },
];

const INDENT_OPTIONS: { value: JsonIndent; label: string }[] = [
  { value: '2', label: '2 spaces' },
  { value: '4', label: '4 spaces' },
  { value: 'tab', label: 'Tab' },
];

const Divider = () => <div className="h-4 w-px bg-border" />;

export const JsonToolkit: React.FC = () => {
  const { jsonMode: mode, setJsonMode: setMode } = useUtilitiesStore();

  return (
    <div className="h-full flex flex-col gap-3 text-xs overflow-hidden">
      {mode === 'format' ? (
        <FormatView modeSwitch={<SegmentedControl options={MODE_OPTIONS} value={mode} onChange={setMode} />} />
      ) : (
        <DiffView modeSwitch={<SegmentedControl options={MODE_OPTIONS} value={mode} onChange={setMode} />} />
      )}
    </div>
  );
};

const OptionsBar: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="h-10 px-2.5 bg-surface border border-border rounded-lg flex items-center gap-3 shrink-0 select-none">
    {children}
  </div>
);

const FormatView: React.FC<{ modeSwitch: React.ReactNode }> = ({ modeSwitch }) => {
  const {
    jsonInput: input,
    setJsonInput: setInput,
    jsonOperation: operation,
    setJsonOperation: setOperation,
    jsonIndent: indent,
    setJsonIndent: setIndent,
    jsonSortKeys: sortKeys,
    setJsonSortKeys: setSortKeys,
  } = useUtilitiesStore();
  const { copiedKey, copy } = useCopyFeedback();

  const result = useMemo(() => formatJson(input, { operation, indent, sortKeys }), [input, operation, indent, sortKeys]);
  const summary = useMemo(() => {
    if (!input.trim() || operation === 'escape' || operation === 'unescape') return null;
    const parsed = parseJson(input);
    return parsed.ok ? describeJson(parsed.value) : null;
  }, [input, operation]);

  const takesJson = operation === 'pretty' || operation === 'minify';
  const hasOutput = !!result.output;

  const handlePaste = async () => {
    const text = await readClipboard();
    if (text) setInput(text);
  };

  return (
    <>
      <OptionsBar>
        {modeSwitch}
        <Divider />
        <SegmentedControl options={OPERATION_OPTIONS} value={operation} onChange={setOperation} />
        {(operation === 'pretty' || operation === 'unescape') && (
          <Select value={indent} onChange={(e) => setIndent(e.target.value as JsonIndent)} options={INDENT_OPTIONS} title="Indent" />
        )}
        {takesJson && (
          <label className="flex items-center gap-1.5 text-2xs text-muted-foreground cursor-pointer">
            <Switch checked={sortKeys} onChange={() => setSortKeys(!sortKeys)} />
            Sort keys
          </label>
        )}
      </OptionsBar>

      <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-3">
        <ToolPane
          title="Input"
          status={result.error ? <StatusPill tone="error">Invalid</StatusPill> : null}
          actions={
            <>
              <PaneAction
                icon="refresh_line"
                label="Sample"
                title="Load a sample"
                onClick={() => setInput(operation === 'unescape' ? JSON.stringify(JSON.stringify(SAMPLE_JSON)) : JSON.stringify(SAMPLE_JSON))}
              />
              <PaneAction icon="clipboard_line" label="Paste" title="Paste from clipboard" onClick={handlePaste} />
              <PaneAction icon="delete_2_line" title="Clear" onClick={() => setInput('')} disabled={!input} />
            </>
          }
          footer={
            <>
              <span>{textStats(input)}</span>
              {summary && <span className="ml-auto">{summary}</span>}
            </>
          }
        >
          <div className="flex-1 min-h-0">
            <CodeEditor value={input} onChange={setInput} language={takesJson ? 'json' : 'plaintext'} bare />
          </div>
          {result.error && <ErrorStrip message={result.error} />}
        </ToolPane>

        <ToolPane
          title="Output"
          actions={
            <>
              <PaneAction icon="swap_line" label="Use as input" title="Replace input with output" onClick={() => setInput(result.output)} disabled={!hasOutput} />
              <PaneAction
                icon="copy_line"
                label="Copy"
                title="Copy output"
                onClick={() => copy(result.output, 'output')}
                disabled={!hasOutput}
                active={copiedKey === 'output'}
              />
            </>
          }
          footer={hasOutput ? textStats(result.output) : null}
        >
          <div className="flex-1 min-h-0">
            <CodeEditor value={result.output} language={operation === 'escape' ? 'plaintext' : 'json'} readOnly bare />
          </div>
        </ToolPane>
      </div>
    </>
  );
};

const CHANGE_STYLES: Record<ChangeKind, { sign: string; className: string }> = {
  added: { sign: '+', className: 'text-emerald-500' },
  removed: { sign: '−', className: 'text-rose-500' },
  changed: { sign: '~', className: 'text-amber-500' },
};

const DiffView: React.FC<{ modeSwitch: React.ReactNode }> = ({ modeSwitch }) => {
  const {
    jsonDiffLeft: left,
    setJsonDiffLeft: setLeft,
    jsonDiffRight: right,
    setJsonDiffRight: setRight,
    jsonDiffInline: inline,
    setJsonDiffInline: setInline,
    jsonIndent: indent,
  } = useUtilitiesStore();

  const parsedLeft = useMemo(() => (left.trim() ? parseJson(left) : null), [left]);
  const parsedRight = useMemo(() => (right.trim() ? parseJson(right) : null), [right]);

  const changes = useMemo(
    () => (parsedLeft?.ok && parsedRight?.ok ? diffJson(parsedLeft.value, parsedRight.value) : null),
    [parsedLeft, parsedRight]
  );

  const counts = useMemo(() => {
    const c: Record<ChangeKind, number> = { added: 0, removed: 0, changed: 0 };
    changes?.forEach((ch) => c[ch.kind]++);
    return c;
  }, [changes]);

  const errors = [
    parsedLeft && !parsedLeft.ok ? `Left: ${parsedLeft.error}` : null,
    parsedRight && !parsedRight.ok ? `Right: ${parsedRight.error}` : null,
  ].filter(Boolean) as string[];

  const formatBoth = () => {
    if (parsedLeft?.ok) setLeft(prettify(parsedLeft.value, indent, true));
    if (parsedRight?.ok) setRight(prettify(parsedRight.value, indent, true));
  };

  const swapSides = () => {
    setLeft(right);
    setRight(left);
  };

  const loadSample = () => {
    setLeft(prettify(SAMPLE_DIFF.left, indent));
    setRight(prettify(SAMPLE_DIFF.right, indent));
  };

  return (
    <>
      <OptionsBar>
        {modeSwitch}
        <Divider />
        <PaneAction
          icon="code_line"
          label="Format both"
          title="Pretty-print and sort keys on both sides"
          onClick={formatBoth}
          disabled={!parsedLeft?.ok && !parsedRight?.ok}
        />
        <label className="flex items-center gap-1.5 text-2xs text-muted-foreground cursor-pointer">
          <Switch checked={inline} onChange={() => setInline(!inline)} />
          Inline
        </label>
      </OptionsBar>

      <div className="flex-1 min-h-0 flex gap-3">
        <ToolPane
          title="Left ⟷ Right"
          className="flex-1 min-w-0"
          actions={
            <>
              <PaneAction icon="refresh_line" label="Sample" title="Load a sample" onClick={loadSample} />
              <PaneAction icon="swap_line" label="Swap" title="Swap sides" onClick={swapSides} disabled={!left && !right} />
              <PaneAction
                icon="delete_2_line"
                title="Clear both"
                onClick={() => {
                  setLeft('');
                  setRight('');
                }}
                disabled={!left && !right}
              />
            </>
          }
        >
          <div className="flex-1 min-h-0">
            <DiffCodeEditor original={left} modified={right} onOriginalChange={setLeft} onModifiedChange={setRight} inline={inline} />
          </div>
          {errors.map((err) => (
            <ErrorStrip key={err} message={err} />
          ))}
        </ToolPane>

        <ToolPane
          title="Changes"
          className="w-80 shrink-0"
          status={
            changes &&
            (changes.length === 0 ? (
              <StatusPill tone="ok">Identical</StatusPill>
            ) : (
              <span className="flex items-center gap-2 text-3xs font-mono">
                {(['added', 'removed', 'changed'] as const).map(
                  (kind) =>
                    counts[kind] > 0 && (
                      <span key={kind} className={CHANGE_STYLES[kind].className}>
                        {CHANGE_STYLES[kind].sign}
                        {counts[kind]}
                      </span>
                    )
                )}
              </span>
            ))
          }
        >
          <div className="flex-1 min-h-0 overflow-y-auto">
            {changes?.map((change, i) => <ChangeRow key={`${change.path}-${i}`} change={change} />)}
          </div>
        </ToolPane>
      </div>
    </>
  );
};

const ChangeRow: React.FC<{ change: JsonChange }> = ({ change }) => {
  const style = CHANGE_STYLES[change.kind];
  return (
    <div className="px-2.5 py-1.5 border-b border-border font-mono text-2xs space-y-0.5 select-text">
      <div className="flex items-baseline gap-1.5">
        <span className={`font-bold ${style.className}`}>{style.sign}</span>
        <span className="text-foreground break-all">{change.path}</span>
      </div>
      {change.kind !== 'added' && (
        <div className="pl-3.5 text-rose-500/90 break-all">{previewValue(change.before)}</div>
      )}
      {change.kind !== 'removed' && (
        <div className="pl-3.5 text-emerald-500/90 break-all">{previewValue(change.after)}</div>
      )}
    </div>
  );
};
