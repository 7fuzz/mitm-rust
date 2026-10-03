import React, { useCallback, useEffect, useMemo } from 'react';
import CryptoJS from 'crypto-js';
import bcrypt from 'bcryptjs';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUtilitiesStore, HashAlgorithm, HashMode } from '../../../stores/useUtilitiesStore';
import { SegmentedControl, Select } from '../../common/ui';
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

const ALGORITHM_GROUPS: AlgorithmGroup<HashAlgorithm>[] = [
  { label: 'Password', items: [{ id: 'bcrypt', name: 'Bcrypt' }] },
  {
    label: 'Digest',
    items: [
      { id: 'md5', name: 'MD5', hint: '128' },
      { id: 'sha1', name: 'SHA-1', hint: '160' },
      { id: 'sha256', name: 'SHA-256', hint: '256' },
      { id: 'sha384', name: 'SHA-384', hint: '384' },
      { id: 'sha512', name: 'SHA-512', hint: '512' },
    ],
  },
  {
    label: 'HMAC',
    items: [
      { id: 'hmac-sha256', name: 'HMAC-SHA256' },
      { id: 'hmac-md5', name: 'HMAC-MD5' },
    ],
  },
];

const MODE_OPTIONS: { value: HashMode; label: string }[] = [
  { value: 'generate', label: 'Generate' },
  { value: 'verify', label: 'Verify' },
];

const COST_OPTIONS = [4, 6, 8, 10, 11, 12, 14].map((n) => ({ value: String(n), label: `Cost ${n}` }));

const BCRYPT_PATTERN = /^\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}$/;

const digest = (algorithm: HashAlgorithm, text: string, hmacKey: string): string => {
  switch (algorithm) {
    case 'md5':
      return CryptoJS.MD5(text).toString();
    case 'sha1':
      return CryptoJS.SHA1(text).toString();
    case 'sha256':
      return CryptoJS.SHA256(text).toString();
    case 'sha384':
      return CryptoJS.SHA384(text).toString();
    case 'sha512':
      return CryptoJS.SHA512(text).toString();
    case 'hmac-sha256':
      return CryptoJS.HmacSHA256(text, hmacKey).toString();
    case 'hmac-md5':
      return CryptoJS.HmacMD5(text, hmacKey).toString();
    default:
      return '';
  }
};

type Verdict = { status: 'match' | 'mismatch' | 'invalid'; computed?: string } | null;

export const HashingTool: React.FC = () => {
  const {
    hashingInput: input,
    setHashingInput: setInput,
    hashingAlgorithm: algorithm,
    setHashingAlgorithm: setAlgorithm,
    hashingMode: mode,
    setHashingMode: setMode,
    hashingVerifyTarget: target,
    setHashingVerifyTarget: setTarget,
    hashingBcryptRounds: bcryptRounds,
    setHashingBcryptRounds: setBcryptRounds,
    hashingBcryptSalt: bcryptSalt,
    setHashingBcryptSalt: setBcryptSalt,
    hashingCase: hashCase,
    setHashingCase: setHashCase,
    hashingHmacKey: hmacKey,
    setHashingHmacKey: setHmacKey,
  } = useUtilitiesStore();
  const { copiedKey, copy } = useCopyFeedback();

  const isBcrypt = algorithm === 'bcrypt';
  const isHmac = algorithm.startsWith('hmac');

  const regenerateSalt = useCallback(
    (rounds: number) => {
      try {
        setBcryptSalt(bcrypt.genSaltSync(rounds));
      } catch {}
    },
    [setBcryptSalt]
  );

  useEffect(() => {
    if (!bcryptSalt) regenerateSalt(bcryptRounds);
  }, [bcryptSalt, bcryptRounds, regenerateSalt]);

  const { output, error } = useMemo(() => {
    if (!input || mode !== 'generate') return { output: '', error: null };
    try {
      if (isBcrypt) return { output: bcrypt.hashSync(input, bcryptSalt || bcrypt.genSaltSync(bcryptRounds)), error: null };
      const hex = digest(algorithm, input, hmacKey);
      return { output: hashCase === 'upper' ? hex.toUpperCase() : hex, error: null };
    } catch (err: any) {
      return { output: '', error: err.message || String(err) };
    }
  }, [input, mode, isBcrypt, algorithm, bcryptSalt, bcryptRounds, hmacKey, hashCase]);

  const verdict = useMemo((): Verdict => {
    const trimmed = target.trim();
    if (mode !== 'verify' || !input || !trimmed) return null;
    if (isBcrypt) {
      if (!BCRYPT_PATTERN.test(trimmed)) return { status: 'invalid' };
      try {
        return { status: bcrypt.compareSync(input, trimmed) ? 'match' : 'mismatch' };
      } catch {
        return { status: 'invalid' };
      }
    }
    const computed = digest(algorithm, input, hmacKey);
    return { status: computed.toLowerCase() === trimmed.toLowerCase() ? 'match' : 'mismatch', computed };
  }, [mode, input, target, isBcrypt, algorithm, hmacKey]);

  const handleRoundsChange = (rounds: number) => {
    setBcryptRounds(rounds);
    regenerateSalt(rounds);
  };

  const handleSendToVerify = () => {
    setTarget(output);
    setMode('verify');
  };

  const loadSample = () => {
    const sample = isBcrypt ? 'SuperSecretPassword2026!' : 'admin:SuperSecretKey2026!';
    setInput(sample);
    if (mode === 'verify') {
      setTarget(isBcrypt ? '$2b$10$D4M2CPJGBmHfdHW79ibZKuH3zgXEVFnA7Lmak254DD8LZe.sX/OCa' : digest(algorithm, sample, hmacKey));
    }
  };

  const pasteInto = async (setter: (v: string) => void) => {
    const text = await readClipboard();
    if (text) setter(text.trim());
  };

  return (
    <div className="h-full flex gap-3 text-xs overflow-hidden">
      <AlgorithmList groups={ALGORITHM_GROUPS} value={algorithm} onChange={setAlgorithm} />

      <div className="flex-1 min-w-0 flex flex-col gap-3">
        <div className="h-10 px-2.5 bg-surface border border-border rounded-lg flex items-center gap-3 shrink-0 select-none">
          <SegmentedControl options={MODE_OPTIONS} value={mode} onChange={setMode} />

          {isBcrypt && mode === 'generate' && (
            <>
              <div className="h-4 w-px bg-border" />
              <Select
                value={String(bcryptRounds)}
                onChange={(e) => handleRoundsChange(Number(e.target.value))}
                options={COST_OPTIONS}
                title="Work factor (2^cost iterations)"
              />
              <div className="flex items-center gap-1 min-w-0">
                <span className="text-3xs uppercase tracking-wider text-muted-foreground">Salt</span>
                <input
                  type="text"
                  value={bcryptSalt}
                  onChange={(e) => setBcryptSalt(e.target.value)}
                  spellCheck={false}
                  className="w-60 min-w-0 bg-background border border-border rounded px-2 py-1 font-mono text-2xs text-foreground focus:outline-none focus:border-primary"
                />
                <PaneAction icon="refresh_line" title="New random salt" onClick={() => regenerateSalt(bcryptRounds)} />
              </div>
            </>
          )}

          {isHmac && (
            <>
              <div className="h-4 w-px bg-border" />
              <div className="flex items-center gap-1.5">
                <MingCuteIcon name="key_line" size={13} className="text-muted-foreground" />
                <input
                  type="text"
                  value={hmacKey}
                  onChange={(e) => setHmacKey(e.target.value)}
                  placeholder="Secret key"
                  spellCheck={false}
                  className="w-48 bg-background border border-border rounded px-2 py-1 font-mono text-2xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                />
              </div>
            </>
          )}

          {!isBcrypt && mode === 'generate' && (
            <SegmentedControl
              className="ml-auto"
              options={[
                { value: 'lower', label: 'Lower' },
                { value: 'upper', label: 'Upper' },
              ]}
              value={hashCase}
              onChange={setHashCase}
            />
          )}
        </div>

        <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-3">
          <ToolPane
            title={mode === 'verify' ? 'Candidate' : 'Input'}
            actions={
              <>
                <PaneAction icon="refresh_line" label="Sample" title="Load a sample" onClick={loadSample} />
                <PaneAction icon="clipboard_line" label="Paste" title="Paste from clipboard" onClick={() => pasteInto(setInput)} />
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

          {mode === 'generate' ? (
            <ToolPane
              title="Hash"
              actions={
                <>
                  <PaneAction icon="check_line" label="Verify" title="Verify against this hash" onClick={handleSendToVerify} disabled={!output} />
                  <PaneAction
                    icon="copy_line"
                    label="Copy"
                    title="Copy hash"
                    onClick={() => copy(output, 'hash')}
                    disabled={!output}
                    active={copiedKey === 'hash'}
                  />
                </>
              }
              footer={output ? (isBcrypt ? <BcryptLegend /> : `${output.length * 4} bits · ${output.length} hex chars`) : null}
            >
              <div className="flex-1 min-h-0 overflow-y-auto p-2.5 font-mono text-xs leading-relaxed break-all select-text">
                {isBcrypt && output ? <BcryptHash hash={output} /> : <span className="text-foreground">{output}</span>}
              </div>
              {error && <ErrorStrip message={error} />}
            </ToolPane>
          ) : (
            <ToolPane
              title="Expected hash"
              status={verdict && <VerdictPill status={verdict.status} />}
              actions={
                <>
                  <PaneAction icon="clipboard_line" label="Paste" title="Paste from clipboard" onClick={() => pasteInto(setTarget)} />
                  <PaneAction icon="delete_2_line" title="Clear" onClick={() => setTarget('')} disabled={!target} />
                </>
              }
            >
              <textarea
                value={target}
                onChange={(e) => setTarget(e.target.value)}
                placeholder={isBcrypt ? '$2b$10$…' : undefined}
                spellCheck={false}
                autoCapitalize="none"
                autoComplete="off"
                className={editorClass}
              />
              {verdict && <VerdictStrip verdict={verdict} />}
            </ToolPane>
          )}
        </div>
      </div>
    </div>
  );
};

const BCRYPT_SEGMENT_COLORS = {
  prefix: 'text-muted-foreground',
  cost: 'text-amber-500',
  salt: 'text-sky-500',
  checksum: 'text-foreground',
};

const BcryptHash: React.FC<{ hash: string }> = ({ hash }) => (
  <>
    <span className={BCRYPT_SEGMENT_COLORS.prefix}>{hash.slice(0, 4)}</span>
    <span className={BCRYPT_SEGMENT_COLORS.cost}>{hash.slice(4, 6)}</span>
    <span className={BCRYPT_SEGMENT_COLORS.prefix}>{hash.slice(6, 7)}</span>
    <span className={BCRYPT_SEGMENT_COLORS.salt}>{hash.slice(7, 29)}</span>
    <span className={BCRYPT_SEGMENT_COLORS.checksum}>{hash.slice(29)}</span>
  </>
);

const BcryptLegend: React.FC = () => (
  <>
    {(
      [
        ['cost', 'cost'],
        ['salt', 'salt'],
        ['checksum', 'hash'],
      ] as const
    ).map(([key, label]) => (
      <span key={key} className="flex items-center gap-1">
        <span className={`${BCRYPT_SEGMENT_COLORS[key]} text-sm leading-none`}>●</span>
        {label}
      </span>
    ))}
  </>
);

const VerdictPill: React.FC<{ status: NonNullable<Verdict>['status'] }> = ({ status }) =>
  status === 'match' ? (
    <StatusPill tone="ok">Match</StatusPill>
  ) : status === 'mismatch' ? (
    <StatusPill tone="error">No match</StatusPill>
  ) : (
    <StatusPill tone="warn">Not a bcrypt hash</StatusPill>
  );

const VerdictStrip: React.FC<{ verdict: NonNullable<Verdict> }> = ({ verdict }) => {
  if (verdict.status === 'invalid') return null;
  const match = verdict.status === 'match';
  return (
    <div
      className={`px-2.5 py-2 border-t flex items-start gap-2 shrink-0 ${
        match ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-500' : 'border-rose-500/30 bg-rose-500/10 text-rose-500'
      }`}
    >
      <MingCuteIcon name={match ? 'check_line' : 'close_line'} size={15} className="mt-px shrink-0" />
      <div className="min-w-0 space-y-0.5">
        <div className="text-xs font-semibold">{match ? 'Hash matches' : 'Hash does not match'}</div>
        {!match && verdict.computed && (
          <div className="text-2xs font-mono text-muted-foreground break-all">Got {verdict.computed}</div>
        )}
      </div>
    </div>
  );
};
