import React, { useState, useMemo, useCallback, useEffect } from 'react';
import CryptoJS from 'crypto-js';
import bcrypt from 'bcryptjs';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUtilitiesStore, HashAlgorithm } from '../../../stores/useUtilitiesStore';

interface HashConfig {
  id: HashAlgorithm;
  name: string;
  isPasswordHash: boolean;
}

const HASH_ALGORITHMS: HashConfig[] = [
  { id: 'bcrypt', name: 'Bcrypt (Password Hash)', isPasswordHash: true },
  { id: 'sha256', name: 'SHA-256 (256-bit)', isPasswordHash: false },
  { id: 'md5', name: 'MD5 (128-bit)', isPasswordHash: false },
  { id: 'sha1', name: 'SHA-1 (160-bit)', isPasswordHash: false },
  { id: 'sha384', name: 'SHA-384 (384-bit)', isPasswordHash: false },
  { id: 'sha512', name: 'SHA-512 (512-bit)', isPasswordHash: false },
  { id: 'hmac-sha256', name: 'HMAC-SHA256', isPasswordHash: false },
  { id: 'hmac-md5', name: 'HMAC-MD5', isPasswordHash: false },
];

export const HashingTool: React.FC = () => {
  const {
    hashingInput: inputText,
    setHashingInput: setInputText,
    hashingAlgorithm: algorithm,
    setHashingAlgorithm: setAlgorithm,
    hashingMode: mode,
    setHashingMode: setMode,
    hashingVerifyTarget: verifyTargetHash,
    setHashingVerifyTarget: setVerifyTargetHash,
    hashingBcryptRounds: bcryptRounds,
    setHashingBcryptRounds: setBcryptRounds,
    hashingBcryptSalt: bcryptSalt,
    setHashingBcryptSalt: setBcryptSalt,
    hashingCase: hashCase,
    setHashingCase: setHashCase,
    hashingHmacKey: hmacKey,
    setHashingHmacKey: setHmacKey,
  } = useUtilitiesStore();

  // User feedback states
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [actionNotice, setActionNotice] = useState<string | null>(null);

  const selectedAlgoConfig = useMemo(
    () => HASH_ALGORITHMS.find((a) => a.id === algorithm) || HASH_ALGORITHMS[0],
    [algorithm]
  );

  // Generate initial or new salt for bcrypt
  const generateNewBcryptSalt = useCallback((rounds = bcryptRounds) => {
    try {
      const generated = bcrypt.genSaltSync(rounds);
      setBcryptSalt(generated);
      return generated;
    } catch {
      return '';
    }
  }, [bcryptRounds]);

  useEffect(() => {
    if (!bcryptSalt) {
      generateNewBcryptSalt(10);
    }
  }, [bcryptSalt, generateNewBcryptSalt]);

  const handleRoundsChange = (newRounds: number) => {
    setBcryptRounds(newRounds);
    generateNewBcryptSalt(newRounds);
    triggerNotice(`Salt regenerated (${newRounds} rounds)`);
  };

  const copyWithFeedback = useCallback((text: string, key = 'output') => {
    if (!text) return;
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 1800);
  }, []);

  const triggerNotice = useCallback((msg: string) => {
    setActionNotice(msg);
    setTimeout(() => setActionNotice(null), 2000);
  }, []);

  // Compute Standard Hash or Bcrypt for candidate text
  const computeDigest = useCallback(
    (textToHash: string): string => {
      if (!textToHash) return '';
      switch (algorithm) {
        case 'md5':
          return CryptoJS.MD5(textToHash).toString();
        case 'sha1':
          return CryptoJS.SHA1(textToHash).toString();
        case 'sha256':
          return CryptoJS.SHA256(textToHash).toString();
        case 'sha384':
          return CryptoJS.SHA384(textToHash).toString();
        case 'sha512':
          return CryptoJS.SHA512(textToHash).toString();
        case 'hmac-sha256':
          return CryptoJS.HmacSHA256(textToHash, hmacKey).toString();
        case 'hmac-md5':
          return CryptoJS.HmacMD5(textToHash, hmacKey).toString();
        default:
          return '';
      }
    },
    [algorithm, hmacKey]
  );

  // Generation Engine
  const { outputText, errorMsg, bcryptDetails } = useMemo(() => {
    if (!inputText) {
      return { outputText: '', errorMsg: null, bcryptDetails: null };
    }

    try {
      if (algorithm === 'bcrypt') {
        const effectiveSalt = bcryptSalt || bcrypt.genSaltSync(bcryptRounds);
        const hash = bcrypt.hashSync(inputText, effectiveSalt);
        let details = null;
        try {
          const rounds = bcrypt.getRounds(hash);
          const salt = bcrypt.getSalt(hash);
          const version = hash.slice(0, 4);
          const checksum = hash.slice(29);
          details = {
            version,
            rounds,
            salt,
            saltOnly: salt.slice(7),
            checksum,
          };
        } catch {}

        return {
          outputText: hash,
          errorMsg: null,
          bcryptDetails: details,
        };
      }

      const digest = computeDigest(inputText);
      return {
        outputText: hashCase === 'upper' ? digest.toUpperCase() : digest.toLowerCase(),
        errorMsg: null,
        bcryptDetails: null,
      };
    } catch (err: any) {
      return {
        outputText: '',
        errorMsg: `Hashing Error: ${err.message || String(err)}`,
        bcryptDetails: null,
      };
    }
  }, [inputText, algorithm, bcryptSalt, bcryptRounds, hashCase, computeDigest]);

  // Verification Engine
  const verificationResult = useMemo(() => {
    if (mode !== 'verify') return null;
    const target = verifyTargetHash.trim();
    if (!inputText || !target) {
      return {
        status: 'idle',
        message: 'Enter candidate text on the left and paste the target hash on the right.',
      };
    }

    if (algorithm === 'bcrypt') {
      const isFormatValid = /^\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}$/.test(target);
      if (!isFormatValid) {
        return {
          status: 'invalid',
          message: 'Invalid Bcrypt hash format. Standard format is 60 characters starting with $2a$, $2b$, or $2y$.',
        };
      }

      try {
        const isMatch = bcrypt.compareSync(inputText, target);
        let rounds: number | null = null;
        let salt: string | null = null;
        try {
          rounds = bcrypt.getRounds(target);
          salt = bcrypt.getSalt(target);
        } catch {}

        return {
          status: isMatch ? 'match' : 'mismatch',
          isMatch,
          rounds,
          salt,
          message: isMatch
            ? 'MATCH CONFIRMED: Candidate text matches the target Bcrypt hash.'
            : 'VERIFICATION FAILED: Candidate text does NOT match the target Bcrypt hash.',
        };
      } catch (e: any) {
        return {
          status: 'error',
          message: `Verification Error: ${e.message}`,
        };
      }
    } else {
      const computed = computeDigest(inputText);
      const isMatch = computed.toLowerCase() === target.toLowerCase();
      return {
        status: isMatch ? 'match' : 'mismatch',
        isMatch,
        computed,
        message: isMatch
          ? `MATCH CONFIRMED: Candidate text produces the exact ${selectedAlgoConfig.name} digest.`
          : `VERIFICATION FAILED: Candidate text digest does not match the target ${selectedAlgoConfig.name}.`,
      };
    }
  }, [mode, verifyTargetHash, inputText, algorithm, computeDigest, selectedAlgoConfig]);

  // Actions
  const handleSendToVerify = () => {
    if (!outputText) return;
    setVerifyTargetHash(outputText);
    setMode('verify');
    triggerNotice('Hash loaded into Verifier');
  };

  const handlePasteInput = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setInputText(text);
        triggerNotice('Pasted from clipboard');
      }
    } catch {}
  };

  const handlePasteTargetHash = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setVerifyTargetHash(text.trim());
        triggerNotice('Pasted target hash');
      }
    } catch {}
  };

  const handleLoadSample = () => {
    if (algorithm === 'bcrypt') {
      setInputText('SuperSecretPassword2026!');
      if (mode === 'verify') {
        setVerifyTargetHash('$2b$10$D4M2CPJGBmHfdHW79ibZKuH3zgXEVFnA7Lmak254DD8LZe.sX/OCa');
      }
    } else {
      setInputText('admin:SuperSecretKey2026!');
      if (mode === 'verify') {
        setVerifyTargetHash(computeDigest('admin:SuperSecretKey2026!'));
      }
    }
    triggerNotice('Sample loaded');
  };

  const getStats = (str: string) => {
    const chars = str.length;
    const bytes = new TextEncoder().encode(str).length;
    const lines = str ? str.split('\n').length : 0;
    return `${chars} chars · ${bytes} B · ${lines} line${lines === 1 ? '' : 's'}`;
  };

  return (
    <div className="h-full flex flex-col gap-3 text-xs overflow-hidden select-none">
      {/* Top Toolbar */}
      <div className="bg-surface border border-border rounded-lg p-3 flex flex-wrap items-center justify-between gap-3 shrink-0 shadow-2xs">
        {/* Left Side: Algorithm Selector & Mode Switcher */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Algorithm Dropdown */}
          <div className="flex items-center gap-1.5">
            <span className="font-semibold text-muted-foreground uppercase text-2xs tracking-wider">
              Algorithm:
            </span>
            <select
              value={algorithm}
              onChange={(e) => {
                const nextAlgo = e.target.value as HashAlgorithm;
                setAlgorithm(nextAlgo);
                if (nextAlgo === 'bcrypt' && !bcryptSalt) {
                  generateNewBcryptSalt(10);
                }
              }}
              className="bg-background border border-border rounded px-2.5 py-1.5 font-semibold text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer hover:border-muted-foreground transition-colors"
            >
              <optgroup label="Password Hashing">
                <option value="bcrypt">Bcrypt (Password Hash)</option>
              </optgroup>
              <optgroup label="Cryptographic Digests">
                <option value="sha256">SHA-256 (256-bit)</option>
                <option value="md5">MD5 (128-bit)</option>
                <option value="sha1">SHA-1 (160-bit)</option>
                <option value="sha384">SHA-384 (384-bit)</option>
                <option value="sha512">SHA-512 (512-bit)</option>
              </optgroup>
              <optgroup label="HMAC (Keyed Hash)">
                <option value="hmac-sha256">HMAC-SHA256</option>
                <option value="hmac-md5">HMAC-MD5</option>
              </optgroup>
            </select>
          </div>

          {/* Quick Filter Chips */}
          <div className="hidden lg:flex items-center gap-1 border-l border-border pl-2">
            {(['bcrypt', 'sha256', 'md5', 'sha1', 'hmac-sha256'] as HashAlgorithm[]).map((algoKey) => (
              <button
                key={algoKey}
                onClick={() => {
                  setAlgorithm(algoKey);
                  if (algoKey === 'bcrypt' && !bcryptSalt) {
                    generateNewBcryptSalt(10);
                  }
                }}
                className={`px-2 py-1 rounded text-2xs font-medium transition-all ${
                  algorithm === algoKey
                    ? 'bg-neutral text-foreground font-bold shadow-2xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
                }`}
              >
                {algoKey.toUpperCase()}
              </button>
            ))}
          </div>

          {/* Mode Switcher: Generate vs Verify */}
          <div className="flex items-center border-l border-border pl-2">
            <div className="flex items-center bg-background border border-border rounded p-0.5 shadow-2xs">
              <button
                onClick={() => setMode('generate')}
                className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-semibold transition-all ${
                  mode === 'generate'
                    ? 'bg-primary text-primary-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-surface'
                }`}
                title="Generate hash from text"
              >
                <MingCuteIcon name="hash_line" size={12} />
                <span>Generate Hash</span>
              </button>
              <button
                onClick={() => setMode('verify')}
                className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-semibold transition-all ${
                  mode === 'verify'
                    ? 'bg-primary text-primary-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-surface'
                }`}
                title="Verify candidate text against target hash"
              >
                <MingCuteIcon name="check_line" size={12} />
                <span>Verify Hash</span>
              </button>
            </div>
          </div>
        </div>

        {/* Center / Contextual Options */}
        <div className="flex flex-wrap items-center gap-2">
          {algorithm === 'bcrypt' && mode === 'generate' && (
            <div className="flex items-center gap-2 bg-background border border-border rounded px-2.5 py-1">
              <div className="flex items-center gap-1">
                <span className="text-2xs text-muted-foreground font-medium">Cost (Rounds):</span>
                <select
                  value={bcryptRounds}
                  onChange={(e) => handleRoundsChange(Number(e.target.value))}
                  className="bg-transparent text-xs text-foreground font-mono focus:outline-none cursor-pointer"
                >
                  <option value={4}>4 (Fastest / Dev)</option>
                  <option value={6}>6</option>
                  <option value={8}>8</option>
                  <option value={10}>10 (Default)</option>
                  <option value={11}>11</option>
                  <option value={12}>12 (Secure)</option>
                  <option value={14}>14 (High Security)</option>
                </select>
              </div>

              <div className="flex items-center gap-1 border-l border-border pl-2">
                <span className="text-2xs text-muted-foreground font-medium">Salt:</span>
                <input
                  type="text"
                  value={bcryptSalt}
                  onChange={(e) => setBcryptSalt(e.target.value)}
                  placeholder="Bcrypt salt..."
                  className="bg-transparent font-mono text-xs text-foreground focus:outline-none w-36 truncate"
                  title="Current Bcrypt Salt (editable)"
                />
                <button
                  onClick={() => {
                    generateNewBcryptSalt(bcryptRounds);
                    triggerNotice('New random salt generated');
                  }}
                  className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                  title="Generate new random salt"
                >
                  <MingCuteIcon name="refresh_line" size={12} />
                </button>
              </div>
            </div>
          )}

          {algorithm !== 'bcrypt' && (
            <div className="flex items-center gap-2">
              {algorithm.startsWith('hmac') && (
                <div className="flex items-center gap-1 bg-background border border-border rounded px-2 py-1">
                  <MingCuteIcon name="key_line" size={12} className="text-muted-foreground" />
                  <input
                    type="text"
                    value={hmacKey}
                    onChange={(e) => setHmacKey(e.target.value)}
                    placeholder="HMAC Secret Key"
                    className="bg-transparent font-mono text-xs text-foreground focus:outline-none w-28"
                  />
                </div>
              )}

              {mode === 'generate' && (
                <button
                  onClick={() => setHashCase(hashCase === 'lower' ? 'upper' : 'lower')}
                  className="px-2 py-1 rounded text-2xs font-mono font-bold bg-background text-foreground hover:bg-neutral-subtle border border-border"
                  title="Toggle Digest Case"
                >
                  {hashCase === 'upper' ? 'UPPER' : 'lower'}
                </button>
              )}
            </div>
          )}
        </div>

        {/* Right Side: Quick Actions & Notices */}
        <div className="flex items-center gap-2">
          {actionNotice && (
            <span className="text-2xs font-semibold text-emerald-500 font-mono animate-pulse">
              {actionNotice}
            </span>
          )}

          <button
            onClick={() => {
              setInputText('');
              if (mode === 'verify') setVerifyTargetHash('');
            }}
            disabled={!inputText && (mode !== 'verify' || !verifyTargetHash)}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle border border-transparent hover:border-border transition-colors disabled:opacity-40"
            title="Clear Input"
          >
            <MingCuteIcon name="delete_2_line" size={13} />
            <span className="hidden sm:inline">Clear</span>
          </button>
        </div>
      </div>

      {/* Main Dual-Pane Area */}
      <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-3 overflow-hidden">
        {/* ================= LEFT PANE: INPUT / CANDIDATE ================= */}
        <div className="flex flex-col bg-surface border border-border rounded-lg overflow-hidden shadow-2xs">
          <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0">
            <div className="flex items-center gap-2">
              <span className="px-2 py-0.5 rounded bg-primary text-primary-foreground font-mono font-bold text-2xs tracking-wider">
                {mode === 'verify' ? 'CANDIDATE TEXT' : 'INPUT STRING'}
              </span>
              <span className="text-muted-foreground font-mono text-2xs">
                {getStats(inputText)}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <button
                onClick={handleLoadSample}
                className="px-2 py-1 rounded text-2xs font-medium text-muted-foreground hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors"
                title="Load sample password or string"
              >
                <MingCuteIcon name="refresh_line" size={12} />
                <span>Sample</span>
              </button>

              <button
                onClick={handlePasteInput}
                className="px-2 py-1 rounded text-2xs font-medium text-muted-foreground hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors"
                title="Paste from clipboard"
              >
                <MingCuteIcon name="edit_line" size={12} />
                <span>Paste</span>
              </button>

              {inputText && (
                <button
                  onClick={() => setInputText('')}
                  className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                  title="Clear text"
                >
                  <MingCuteIcon name="close_line" size={13} />
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 p-2 bg-background overflow-hidden relative">
            <textarea
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              placeholder={
                mode === 'verify'
                  ? 'Enter candidate text to test against the hash...'
                  : 'Enter text to hash...'
              }
              spellCheck={false}
              autoCapitalize="none"
              autoComplete="off"
              className="w-full h-full bg-transparent text-foreground font-mono text-xs resize-none focus:outline-none p-1 leading-relaxed selection:bg-primary/20"
            />
          </div>
        </div>

        {/* ================= RIGHT PANE: GENERATED OUTPUT OR TARGET HASH VERIFIER ================= */}
        {mode === 'verify' ? (
          /* ================= VERIFY HASH MODE ================= */
          <div className="flex flex-col bg-surface border border-border rounded-lg overflow-hidden shadow-2xs">
            <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0">
              <div className="flex items-center gap-2">
                <span className="px-2 py-0.5 rounded bg-neutral-subtle border border-border text-foreground font-mono font-bold text-2xs tracking-wider">
                  TARGET HASH TO VERIFY
                </span>
                <span
                  className={`font-semibold text-2xs px-2 py-0.5 rounded ${
                    verificationResult?.status === 'match'
                      ? 'bg-emerald-500/20 text-emerald-500 font-bold'
                      : verificationResult?.status === 'mismatch'
                      ? 'bg-rose-500/20 text-rose-500 font-bold'
                      : verificationResult?.status === 'invalid'
                      ? 'bg-amber-500/20 text-amber-500 font-bold'
                      : 'text-muted-foreground bg-neutral-subtle'
                  }`}
                >
                  {verificationResult?.status === 'match'
                    ? 'MATCH'
                    : verificationResult?.status === 'mismatch'
                    ? 'MISMATCH'
                    : verificationResult?.status === 'invalid'
                    ? 'INVALID HASH'
                    : 'READY'}
                </span>
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  onClick={handlePasteTargetHash}
                  className="px-2 py-1 rounded text-2xs font-medium text-muted-foreground hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors"
                  title="Paste hash from clipboard"
                >
                  <MingCuteIcon name="edit_line" size={12} />
                  <span>Paste Hash</span>
                </button>

                {verifyTargetHash && (
                  <button
                    onClick={() => setVerifyTargetHash('')}
                    className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                    title="Clear target hash"
                  >
                    <MingCuteIcon name="close_line" size={13} />
                  </button>
                )}
              </div>
            </div>

            <div className="flex-1 p-3 bg-background overflow-y-auto flex flex-col gap-3">
              <div className="space-y-1">
                <label className="text-2xs font-bold text-muted-foreground uppercase tracking-wider block">
                  Paste Existing {selectedAlgoConfig.name}:
                </label>
                <textarea
                  value={verifyTargetHash}
                  onChange={(e) => setVerifyTargetHash(e.target.value)}
                  placeholder={`Paste the ${selectedAlgoConfig.name} to verify candidate text against...`}
                  rows={3}
                  spellCheck={false}
                  autoCapitalize="none"
                  autoComplete="off"
                  className="w-full bg-surface border border-border rounded p-2.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary resize-none selection:bg-primary/20"
                />
              </div>

              {/* Verification Outcome Banner */}
              <div
                className={`p-4 rounded-lg border flex items-start gap-3 transition-all ${
                  verificationResult?.status === 'match'
                    ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
                    : verificationResult?.status === 'mismatch'
                    ? 'bg-rose-500/10 border-rose-500/30 text-rose-400'
                    : verificationResult?.status === 'invalid'
                    ? 'bg-amber-500/10 border-amber-500/30 text-amber-400'
                    : 'bg-surface border-border text-muted-foreground'
                }`}
              >
                <div className="mt-0.5 shrink-0">
                  {verificationResult?.status === 'match' ? (
                    <div className="w-7 h-7 rounded-full bg-emerald-500 text-black flex items-center justify-center font-bold">
                      <MingCuteIcon name="check_line" size={16} />
                    </div>
                  ) : verificationResult?.status === 'mismatch' ? (
                    <div className="w-7 h-7 rounded-full bg-rose-500 text-white flex items-center justify-center font-bold">
                      <MingCuteIcon name="close_line" size={16} />
                    </div>
                  ) : (
                    <div className="w-7 h-7 rounded-full bg-neutral text-foreground flex items-center justify-center">
                      <MingCuteIcon name="hash_line" size={15} />
                    </div>
                  )}
                </div>

                <div className="flex-1 space-y-1">
                  <div className="font-bold text-xs">
                    {verificationResult?.status === 'match'
                      ? 'Hash Matches'
                      : verificationResult?.status === 'mismatch'
                      ? 'Hash Does Not Match'
                      : verificationResult?.status === 'invalid'
                      ? 'Invalid Hash Format'
                      : 'Verification Standby'}
                  </div>
                  <div className="text-2xs font-mono opacity-90 leading-relaxed">
                    {verificationResult?.message}
                  </div>

                  {verificationResult?.status === 'match' && algorithm === 'bcrypt' && (
                    <div className="pt-2 mt-2 border-t border-emerald-500/20 grid grid-cols-2 gap-2 text-2xs font-mono">
                      <div>
                        <span className="text-muted-foreground font-sans block">Cost (Rounds):</span>
                        <span className="font-bold text-foreground">
                          {verificationResult.rounds || '10'} (2^{verificationResult.rounds || 10} iterations)
                        </span>
                      </div>
                      <div>
                        <span className="text-muted-foreground font-sans block">Salt Prefix:</span>
                        <span className="font-bold text-foreground truncate block">
                          {verificationResult.salt || 'N/A'}
                        </span>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              <div className="bg-surface border border-border rounded-lg p-3 space-y-1.5 text-2xs text-muted-foreground">
                <span className="font-bold text-foreground block">How verification works:</span>
                <p>
                  {algorithm === 'bcrypt'
                    ? 'Bcrypt incorporates a cryptographic salt and key stretching. Verification uses constant-time extraction of cost and salt from the target hash to verify whether candidate text re-hashes to the identical ciphertext.'
                    : `Constant-time equality comparison between the candidate text's ${selectedAlgoConfig.name} digest and the target hash.`}
                </p>
              </div>
            </div>
          </div>
        ) : (
          /* ================= GENERATE HASH MODE ================= */
          <div className="flex flex-col bg-surface border border-border rounded-lg overflow-hidden shadow-2xs">
            <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0">
              <div className="flex items-center gap-2">
                <span className="px-2 py-0.5 rounded bg-neutral-subtle border border-border text-foreground font-mono font-bold text-2xs tracking-wider">
                  GENERATED HASH
                </span>
                <span
                  className={`font-semibold text-2xs px-1.5 py-0.5 rounded ${
                    errorMsg
                      ? 'bg-rose-500/15 text-rose-500'
                      : outputText
                      ? 'bg-emerald-500/15 text-emerald-500'
                      : 'text-muted-foreground'
                  }`}
                >
                  {errorMsg ? 'Error' : selectedAlgoConfig.name}
                </span>
                {outputText && !errorMsg && (
                  <span className="text-muted-foreground font-mono text-2xs hidden sm:inline">
                    {getStats(outputText)}
                  </span>
                )}
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  onClick={handleSendToVerify}
                  disabled={!outputText || !!errorMsg}
                  className="px-2 py-1 rounded text-2xs font-semibold text-primary hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  title="Send this hash into Verifier mode to test passwords"
                >
                  <MingCuteIcon name="check_line" size={12} />
                  <span>Verify this Hash</span>
                </button>

                <button
                  onClick={() => copyWithFeedback(outputText, 'output')}
                  disabled={!outputText || !!errorMsg}
                  className={`px-2.5 py-1 rounded text-2xs font-semibold flex items-center gap-1 transition-all ${
                    copiedKey === 'output'
                      ? 'bg-emerald-600 text-white shadow-2xs'
                      : 'bg-primary text-primary-foreground hover:bg-primary-hover shadow-2xs disabled:opacity-40 disabled:cursor-not-allowed'
                  }`}
                  title="Copy hash to clipboard"
                >
                  <MingCuteIcon
                    name={copiedKey === 'output' ? 'check_line' : 'copy_line'}
                    size={12}
                  />
                  <span>{copiedKey === 'output' ? 'Copied!' : 'Copy'}</span>
                </button>
              </div>
            </div>

            <div className="flex-1 p-2 bg-background overflow-hidden relative flex flex-col">
              {errorMsg ? (
                <div className="h-full flex flex-col items-center justify-center p-6 text-center">
                  <div className="w-10 h-10 rounded-full bg-rose-500/10 text-rose-500 flex items-center justify-center mb-3">
                    <MingCuteIcon name="warning_line" size={20} />
                  </div>
                  <h4 className="font-bold text-foreground text-xs mb-1">Hashing Failed</h4>
                  <p className="text-rose-500 font-mono text-xs max-w-md break-all leading-relaxed bg-rose-500/5 p-3 rounded border border-rose-500/20 mb-2">
                    {errorMsg}
                  </p>
                </div>
              ) : (
                <div className="h-full flex flex-col gap-2 overflow-y-auto">
                  <textarea
                    readOnly
                    value={outputText}
                    placeholder="Generated hash will appear here..."
                    className="flex-1 w-full bg-transparent text-foreground font-mono text-xs resize-none focus:outline-none p-1 leading-relaxed selection:bg-primary/20 select-all cursor-text min-h-[90px]"
                  />

                  {algorithm === 'bcrypt' && bcryptDetails && (
                    <div className="bg-surface border border-border rounded-lg p-2.5 space-y-2 shrink-0 select-text">
                      <div className="flex items-center justify-between border-b border-border pb-1.5">
                        <span className="font-bold text-foreground text-2xs tracking-wider uppercase flex items-center gap-1.5">
                          <MingCuteIcon name="hash_line" size={12} className="text-primary" />
                          Bcrypt Hash Anatomy
                        </span>
                        <span className="text-muted-foreground text-2xs font-mono">
                          Total: 60 characters
                        </span>
                      </div>

                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-2xs font-mono">
                        <div className="bg-background border border-border rounded p-1.5">
                          <span className="text-muted-foreground font-sans block text-2xs">Prefix:</span>
                          <span className="font-bold text-primary">{bcryptDetails.version}</span>
                        </div>
                        <div className="bg-background border border-border rounded p-1.5">
                          <span className="text-muted-foreground font-sans block text-2xs">Cost (Rounds):</span>
                          <span className="font-bold text-foreground">{bcryptDetails.rounds} (2^{bcryptDetails.rounds})</span>
                        </div>
                        <div className="bg-background border border-border rounded p-1.5">
                          <span className="text-muted-foreground font-sans block text-2xs">Salt (22 chars):</span>
                          <span className="font-bold text-foreground truncate block" title={bcryptDetails.saltOnly}>
                            {bcryptDetails.saltOnly}
                          </span>
                        </div>
                        <div className="bg-background border border-border rounded p-1.5">
                          <span className="text-muted-foreground font-sans block text-2xs">Ciphertext (31 chars):</span>
                          <span className="font-bold text-foreground truncate block" title={bcryptDetails.checksum}>
                            {bcryptDetails.checksum}
                          </span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
