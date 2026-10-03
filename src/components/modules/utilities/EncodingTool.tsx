import React, { useMemo } from 'react';
import { useUtilitiesStore, EncodingAlgorithm, Direction } from '../../../stores/useUtilitiesStore';
import { SegmentedControl, Select, Switch } from '../../common/ui';
import {
  AlgorithmGroup,
  AlgorithmList,
  ErrorStrip,
  PaneAction,
  StatusPill,
  ToolPane,
  editorClass,
  readClipboard,
  textStats,
  useCopyFeedback,
} from './ToolLayout';
import { HexDelimiter, JwtParts, sampleFor, transform } from './encoding';

const ALGORITHM_GROUPS: AlgorithmGroup<EncodingAlgorithm>[] = [
  {
    label: 'Encodings',
    items: [
      { id: 'base64', name: 'Base64' },
      { id: 'base64url', name: 'Base64URL' },
      { id: 'url', name: 'URL' },
      { id: 'hex', name: 'Hex' },
      { id: 'html', name: 'HTML entities' },
      { id: 'binary', name: 'Binary' },
    ],
  },
  { label: 'Tokens', items: [{ id: 'jwt', name: 'JWT' }] },
];

const HEX_DELIMITERS: { value: HexDelimiter; label: string }[] = [
  { value: ' ', label: '4d 49' },
  { value: '', label: '4d49' },
  { value: ':', label: '4d:49' },
  { value: '\\x', label: '\\x4d\\x49' },
  { value: '0x', label: '0x4d 0x49' },
];

const DIRECTION_OPTIONS: { value: Direction; label: string }[] = [
  { value: 'encode', label: 'Encode' },
  { value: 'decode', label: 'Decode' },
];

export const EncodingTool: React.FC = () => {
  const {
    encodingInput: input,
    setEncodingInput: setInput,
    encodingAlgorithm: algorithm,
    setEncodingAlgorithm: setAlgorithm,
    encodingDirection: direction,
    setEncodingDirection: setDirection,
    encodingHexDelimiter: hexDelimiter,
    setEncodingHexDelimiter: setHexDelimiter,
    encodingHexCase: hexCase,
    setEncodingHexCase: setHexCase,
    encodingUrlFull: urlFull,
    setEncodingUrlFull: setUrlFull,
  } = useUtilitiesStore();
  const { copiedKey, copy } = useCopyFeedback();

  const result = useMemo(
    () => transform(input, { algorithm, direction, hexDelimiter, hexCase, urlFull }),
    [input, algorithm, direction, hexDelimiter, hexCase, urlFull]
  );
  const hasOutput = !!result.output && !result.error;
  const canSwap = hasOutput && algorithm !== 'jwt';

  const handleSwap = () => {
    setInput(result.output);
    setDirection(direction === 'encode' ? 'decode' : 'encode');
  };

  const handlePaste = async () => {
    const text = await readClipboard();
    if (text) setInput(text);
  };

  return (
    <div className="h-full flex gap-3 text-xs overflow-hidden">
      <AlgorithmList groups={ALGORITHM_GROUPS} value={algorithm} onChange={setAlgorithm} />

      <div className="flex-1 min-w-0 flex flex-col gap-3">
        <div className="h-10 px-2.5 bg-surface border border-border rounded-lg flex items-center gap-3 shrink-0 select-none">
          <SegmentedControl options={DIRECTION_OPTIONS} value={direction} onChange={setDirection} />

          {algorithm === 'hex' && direction === 'encode' && (
            <>
              <div className="h-4 w-px bg-border" />
              <Select
                value={hexDelimiter}
                onChange={(e) => setHexDelimiter(e.target.value as HexDelimiter)}
                options={HEX_DELIMITERS}
                title="Byte format"
              />
              <SegmentedControl
                options={[
                  { value: 'lower', label: 'Lower' },
                  { value: 'upper', label: 'Upper' },
                ]}
                value={hexCase}
                onChange={setHexCase}
              />
            </>
          )}

          {algorithm === 'url' && direction === 'encode' && (
            <>
              <div className="h-4 w-px bg-border" />
              <label className="flex items-center gap-1.5 text-2xs text-muted-foreground cursor-pointer">
                <Switch checked={urlFull} onChange={() => setUrlFull(!urlFull)} />
                Encode every character
              </label>
            </>
          )}
        </div>

        <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-3">
          <ToolPane
            title="Input"
            actions={
              <>
                <PaneAction icon="refresh_line" label="Sample" title="Load a sample" onClick={() => setInput(sampleFor(algorithm, direction))} />
                <PaneAction icon="clipboard_line" label="Paste" title="Paste from clipboard" onClick={handlePaste} />
                <PaneAction icon="delete_2_line" title="Clear" onClick={() => setInput('')} disabled={!input} />
              </>
            }
            footer={textStats(input)}
          >
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              spellCheck={false}
              autoCapitalize="none"
              autoComplete="off"
              className={editorClass}
            />
          </ToolPane>

          <ToolPane
            title="Output"
            status={result.error ? <StatusPill tone="error">Invalid input</StatusPill> : null}
            actions={
              <>
                <PaneAction icon="swap_line" label="Swap" title="Use output as input and flip direction" onClick={handleSwap} disabled={!canSwap} />
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
            {result.jwt ? (
              <JwtView jwt={result.jwt} copiedKey={copiedKey} onCopy={copy} />
            ) : (
              <textarea readOnly value={result.output} className={`${editorClass} cursor-text`} />
            )}
            {result.error && <ErrorStrip message={result.error} />}
          </ToolPane>
        </div>
      </div>
    </div>
  );
};

const TIME_CLAIMS = ['iat', 'nbf', 'exp'] as const;

const JwtView: React.FC<{
  jwt: JwtParts;
  copiedKey: string | null;
  onCopy: (text: string, key: string) => void;
}> = ({ jwt, copiedKey, onCopy }) => {
  const now = Date.now() / 1000;
  const times = TIME_CLAIMS.filter((k) => typeof jwt.claims?.[k] === 'number').map((k) => ({
    key: k,
    value: jwt.claims![k] as number,
  }));
  const exp = jwt.claims?.exp;
  const expired = typeof exp === 'number' && exp < now;

  return (
    <div className="flex-1 min-h-0 overflow-y-auto p-2.5 space-y-2.5">
      <JwtSection title="Header" accent="text-rose-400" text={jwt.header} copyKey="jwt-header" copiedKey={copiedKey} onCopy={onCopy} />
      <JwtSection
        title="Payload"
        accent="text-violet-400"
        text={jwt.payload}
        copyKey="jwt-payload"
        copiedKey={copiedKey}
        onCopy={onCopy}
        extra={
          times.length > 0 && (
            <div className="px-2.5 py-1.5 border-t border-border flex flex-wrap gap-x-4 gap-y-1 text-3xs font-mono text-muted-foreground">
              {times.map(({ key, value }) => (
                <span key={key}>
                  <span className="text-foreground">{key}</span> {new Date(value * 1000).toLocaleString()}
                </span>
              ))}
              {typeof exp === 'number' && (
                <StatusPill tone={expired ? 'error' : 'ok'}>{expired ? 'Expired' : 'Valid'}</StatusPill>
              )}
            </div>
          )
        }
      />
      <JwtSection
        title="Signature"
        accent="text-sky-400"
        text={jwt.signature || '—'}
        copyKey="jwt-signature"
        copiedKey={copiedKey}
        onCopy={onCopy}
      />
    </div>
  );
};

const JwtSection: React.FC<{
  title: string;
  accent: string;
  text: string;
  copyKey: string;
  copiedKey: string | null;
  onCopy: (text: string, key: string) => void;
  extra?: React.ReactNode;
}> = ({ title, accent, text, copyKey, copiedKey, onCopy, extra }) => (
  <div className="border border-border rounded-md bg-surface overflow-hidden">
    <div className="h-7 pl-2.5 pr-1 border-b border-border flex items-center select-none">
      <span className={`text-3xs font-semibold uppercase tracking-wider ${accent}`}>{title}</span>
      <div className="ml-auto">
        <PaneAction icon="copy_line" title={`Copy ${title.toLowerCase()}`} onClick={() => onCopy(text, copyKey)} active={copiedKey === copyKey} />
      </div>
    </div>
    <pre className="px-2.5 py-2 font-mono text-xs text-foreground whitespace-pre-wrap break-all select-text">{text}</pre>
    {extra}
  </div>
);
