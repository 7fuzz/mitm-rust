import React, { useState, useMemo, useCallback } from 'react';
import CryptoJS from 'crypto-js';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUtilitiesStore, EncodingAlgorithm } from '../../../stores/useUtilitiesStore';

interface EncodingConfig {
  id: EncodingAlgorithm;
  name: string;
  isBidirectional: boolean;
}

const ENCODING_ALGORITHMS: EncodingConfig[] = [
  { id: 'base64', name: 'Base64 (Standard)', isBidirectional: true },
  { id: 'base64url', name: 'Base64URL (RFC 4648)', isBidirectional: true },
  { id: 'url', name: 'URL (Percent-Encoding)', isBidirectional: true },
  { id: 'hex', name: 'Hexadecimal', isBidirectional: true },
  { id: 'html', name: 'HTML Entities', isBidirectional: true },
  { id: 'binary', name: 'Binary (8-bit Bits)', isBidirectional: true },
  { id: 'jwt', name: 'JWT Inspector / Decoder', isBidirectional: false },
];

export const EncodingTool: React.FC = () => {
  const {
    encodingInput: inputText,
    setEncodingInput: setInputText,
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

  // User feedback states
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [actionNotice, setActionNotice] = useState<string | null>(null);

  const selectedAlgoConfig = useMemo(
    () => ENCODING_ALGORITHMS.find((a) => a.id === algorithm) || ENCODING_ALGORITHMS[0],
    [algorithm]
  );

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

  // Transformation Engine
  const { outputText, errorMsg } = useMemo(() => {
    if (!inputText) {
      return { outputText: '', errorMsg: null };
    }

    try {
      if (direction === 'encode') {
        switch (algorithm) {
          case 'base64': {
            const bytes = new TextEncoder().encode(inputText);
            let binary = '';
            for (let i = 0; i < bytes.length; i++) {
              binary += String.fromCharCode(bytes[i]);
            }
            return { outputText: btoa(binary), errorMsg: null };
          }

          case 'base64url': {
            const bytes = new TextEncoder().encode(inputText);
            let binary = '';
            for (let i = 0; i < bytes.length; i++) {
              binary += String.fromCharCode(bytes[i]);
            }
            const b64 = btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
            return { outputText: b64, errorMsg: null };
          }

          case 'url': {
            if (urlFull) {
              const bytes = new TextEncoder().encode(inputText);
              const fullEnc = Array.from(bytes)
                .map((b) => '%' + b.toString(16).toUpperCase().padStart(2, '0'))
                .join('');
              return { outputText: fullEnc, errorMsg: null };
            }
            return { outputText: encodeURIComponent(inputText), errorMsg: null };
          }

          case 'hex': {
            const bytes = new TextEncoder().encode(inputText);
            const hexParts = Array.from(bytes).map((b) => {
              const h = b.toString(16).padStart(2, '0');
              const formatted = hexCase === 'upper' ? h.toUpperCase() : h;
              if (hexDelimiter === '\\x') return '\\x' + formatted;
              if (hexDelimiter === '0x') return '0x' + formatted;
              return formatted;
            });

            if (hexDelimiter === '\\x') {
              return { outputText: hexParts.join(''), errorMsg: null };
            }
            return { outputText: hexParts.join(hexDelimiter), errorMsg: null };
          }

          case 'html': {
            const encoded = inputText.replace(/[&<>"']/g, (m) => {
              switch (m) {
                case '&':
                  return '&amp;';
                case '<':
                  return '&lt;';
                case '>':
                  return '&gt;';
                case '"':
                  return '&quot;';
                case "'":
                  return '&#39;';
                default:
                  return m;
              }
            });
            return { outputText: encoded, errorMsg: null };
          }

          case 'binary': {
            const bytes = new TextEncoder().encode(inputText);
            const binary = Array.from(bytes)
              .map((b) => b.toString(2).padStart(8, '0'))
              .join(' ');
            return { outputText: binary, errorMsg: null };
          }

          case 'jwt': {
            try {
              const json = JSON.parse(inputText);
              const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }))
                .replace(/\+/g, '-')
                .replace(/\//g, '_')
                .replace(/=+$/, '');
              const payload = btoa(JSON.stringify(json))
                .replace(/\+/g, '-')
                .replace(/\//g, '_')
                .replace(/=+$/, '');
              const signature = CryptoJS.HmacSHA256(`${header}.${payload}`, 'secret')
                .toString(CryptoJS.enc.Base64)
                .replace(/\+/g, '-')
                .replace(/\//g, '_')
                .replace(/=+$/, '');
              return { outputText: `${header}.${payload}.${signature}`, errorMsg: null };
            } catch (err: any) {
              return {
                outputText: '',
                errorMsg: 'JWT Encode expects valid JSON input to construct the token payload.',
              };
            }
          }

          default:
            return { outputText: inputText, errorMsg: null };
        }
      } else {
        // Decode direction
        switch (algorithm) {
          case 'base64': {
            let cleaned = inputText.trim().replace(/\s+/g, '');
            while (cleaned.length % 4 !== 0) {
              cleaned += '=';
            }
            if (!/^[A-Za-z0-9+/=]+$/.test(cleaned)) {
              return {
                outputText: '',
                errorMsg: 'Invalid Base64: input contains non-Base64 characters.',
              };
            }
            const binary = atob(cleaned);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            return { outputText: new TextDecoder().decode(bytes), errorMsg: null };
          }

          case 'base64url': {
            let cleaned = inputText.trim().replace(/\s+/g, '').replace(/-/g, '+').replace(/_/g, '/');
            while (cleaned.length % 4 !== 0) {
              cleaned += '=';
            }
            if (!/^[A-Za-z0-9+/=]+$/.test(cleaned)) {
              return {
                outputText: '',
                errorMsg: 'Invalid Base64URL: input contains illegal characters.',
              };
            }
            const binary = atob(cleaned);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            return { outputText: new TextDecoder().decode(bytes), errorMsg: null };
          }

          case 'url': {
            try {
              const decoded = decodeURIComponent(inputText.replace(/\+/g, '%20'));
              return { outputText: decoded, errorMsg: null };
            } catch (e: any) {
              return { outputText: '', errorMsg: `URL Decode Failed: ${e.message}` };
            }
          }

          case 'hex': {
            const cleaned = inputText.trim().replace(/0x|\\x|[:,\s\-_]/gi, '');
            if (!cleaned) return { outputText: '', errorMsg: null };
            if (cleaned.length % 2 !== 0) {
              return {
                outputText: '',
                errorMsg: `Invalid Hex String: odd character count (${cleaned.length} chars). Hex bytes must come in pairs.`,
              };
            }
            if (!/^[0-9a-fA-F]+$/.test(cleaned)) {
              return {
                outputText: '',
                errorMsg: 'Invalid Hex String: contains non-hexadecimal characters.',
              };
            }
            const byteCount = cleaned.length / 2;
            const bytes = new Uint8Array(byteCount);
            for (let i = 0; i < byteCount; i++) {
              bytes[i] = parseInt(cleaned.substr(i * 2, 2), 16);
            }
            return { outputText: new TextDecoder().decode(bytes), errorMsg: null };
          }

          case 'html': {
            const doc = new DOMParser().parseFromString(inputText, 'text/html');
            return { outputText: doc.documentElement.textContent || '', errorMsg: null };
          }

          case 'binary': {
            const cleaned = inputText.replace(/[^01]/g, '');
            if (!cleaned) return { outputText: '', errorMsg: null };
            if (cleaned.length % 8 !== 0) {
              return {
                outputText: '',
                errorMsg: `Binary stream length (${cleaned.length} bits) must be a multiple of 8 bits.`,
              };
            }
            const byteCount = cleaned.length / 8;
            const bytes = new Uint8Array(byteCount);
            for (let i = 0; i < byteCount; i++) {
              bytes[i] = parseInt(cleaned.substr(i * 8, 8), 2);
            }
            return { outputText: new TextDecoder().decode(bytes), errorMsg: null };
          }

          case 'jwt': {
            const parts = inputText.trim().split('.');
            if (parts.length < 2 || parts.length > 3) {
              return {
                outputText: '',
                errorMsg: 'Invalid JWT format: expected 2 or 3 dot-separated segments (Header.Payload.Signature).',
              };
            }
            const decodeSegment = (seg: string) => {
              let b64 = seg.replace(/-/g, '+').replace(/_/g, '/');
              while (b64.length % 4 !== 0) b64 += '=';
              const binary = atob(b64);
              const bytes = new Uint8Array(binary.length);
              for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
              const decodedStr = new TextDecoder().decode(bytes);
              try {
                return JSON.stringify(JSON.parse(decodedStr), null, 2);
              } catch {
                return decodedStr;
              }
            };

            const header = decodeSegment(parts[0]);
            const payload = decodeSegment(parts[1]);
            const signature = parts[2] ? parts[2] : '(No signature)';

            const jwtDump = [
              '/* ================= JWT HEADER ================= */',
              header,
              '',
              '/* ================ JWT PAYLOAD ================ */',
              payload,
              '',
              '/* =============== JWT SIGNATURE =============== */',
              signature,
            ].join('\n');

            return { outputText: jwtDump, errorMsg: null };
          }

          default:
            return { outputText: inputText, errorMsg: null };
        }
      }
    } catch (err: any) {
      return { outputText: '', errorMsg: `Transformation Error: ${err.message || String(err)}` };
    }
  }, [inputText, algorithm, direction, hexDelimiter, hexCase, urlFull]);

  // Actions
  const handleSwap = () => {
    if (!outputText) return;
    setInputText(outputText);
    if (selectedAlgoConfig.isBidirectional) {
      setDirection(direction === 'encode' ? 'decode' : 'encode');
    }
    triggerNotice('Output sent to Input');
  };

  const handleUseAsInput = () => {
    if (!outputText) return;
    setInputText(outputText);
    triggerNotice('Output copied to Input');
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

  const handleLoadSample = () => {
    if (algorithm === 'jwt') {
      setInputText(
        'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyLTEyMyIsIm5hbWUiOiJBZG1pbiBVc2VyIiwicm9sZXMiOlsic2VjdXJpdHlfc3VpdGUiLCJhdWRpdG9yIl0sImlhdCI6MTcwMDAwMDAwMH0.u2fC9M14Y0T_9w7s4e0o3f8c5b1a2d6e7f8a9b0c1d2'
      );
    } else if (algorithm === 'url') {
      if (direction === 'encode') {
        setInputText('https://api.internal.local/v1/auth?client_id=sec-suite&redirect=https://app.dev/#token');
      } else {
        setInputText('https%3A%2F%2Fapi.internal.local%2Fv1%2Fauth%3Fclient_id%3Dsec-suite%26redirect%3Dhttps%3A%2F%2Fapp.dev%2F%23token');
      }
    } else if (algorithm === 'hex') {
      if (direction === 'encode') {
        setInputText('MITM-Developer-Security-Suite-2.0');
      } else {
        setInputText('4d 49 54 4d 2d 44 65 76 65 6c 6f 70 65 72 2d 53 65 63 75 72 69 74 79 2d 53 75 69 74 65 2d 32 2e 30');
      }
    } else if (algorithm === 'base64' || algorithm === 'base64url') {
      if (direction === 'encode') {
        setInputText('MITM-Developer-Security-Suite-2.0');
      } else {
        setInputText('TUlUTS1EZXZlbG9wZXItU2VjdXJpdHktU3VpdGUtMi4w');
      }
    } else if (algorithm === 'html') {
      if (direction === 'encode') {
        setInputText('<div class="alert" data-user="admin & user">Notice: "Access Granted"</div>');
      } else {
        setInputText('&lt;div class=&quot;alert&quot; data-user=&quot;admin &amp; user&quot;&gt;Notice: &quot;Access Granted&quot;&lt;/div&gt;');
      }
    } else if (algorithm === 'binary') {
      if (direction === 'encode') {
        setInputText('MITM');
      } else {
        setInputText('01001101 01001001 01010100 01001101');
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
        {/* Left Side: Algorithm Selector & Quick Switcher */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Format Selector */}
          <div className="flex items-center gap-1.5">
            <span className="font-semibold text-muted-foreground uppercase text-2xs tracking-wider">
              Format:
            </span>
            <select
              value={algorithm}
              onChange={(e) => setAlgorithm(e.target.value as EncodingAlgorithm)}
              className="bg-background border border-border rounded px-2.5 py-1.5 font-semibold text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer hover:border-muted-foreground transition-colors"
            >
              {ENCODING_ALGORITHMS.map((algo) => (
                <option key={algo.id} value={algo.id}>
                  {algo.name}
                </option>
              ))}
            </select>
          </div>

          {/* Quick Filter Chips */}
          <div className="hidden lg:flex items-center gap-1 border-l border-border pl-2">
            {(['base64', 'base64url', 'url', 'hex', 'html', 'jwt'] as EncodingAlgorithm[]).map((algoKey) => (
              <button
                key={algoKey}
                onClick={() => setAlgorithm(algoKey)}
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

          {/* Direction Toggle: Encode vs Decode */}
          <div className="flex items-center border-l border-border pl-2">
            <div className="flex items-center bg-background border border-border rounded p-0.5 shadow-2xs">
              <button
                onClick={() => setDirection('encode')}
                className={`flex items-center gap-1 px-3 py-1 rounded text-xs font-semibold transition-all ${
                  direction === 'encode'
                    ? 'bg-primary text-primary-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-surface'
                }`}
              >
                <MingCuteIcon name="lock_line" size={12} />
                <span>Encode</span>
              </button>
              <button
                onClick={() => setDirection('decode')}
                className={`flex items-center gap-1 px-3 py-1 rounded text-xs font-semibold transition-all ${
                  direction === 'decode'
                    ? 'bg-primary text-primary-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground hover:bg-surface'
                }`}
              >
                <MingCuteIcon name="unlock_line" size={12} />
                <span>Decode</span>
              </button>
            </div>
          </div>
        </div>

        {/* Center / Contextual Options */}
        <div className="flex flex-wrap items-center gap-2">
          {algorithm === 'hex' && (
            <div className="flex items-center gap-1.5 bg-background border border-border rounded px-2 py-1">
              <span className="text-2xs text-muted-foreground font-medium">Delimiter:</span>
              <select
                value={hexDelimiter}
                onChange={(e) => setHexDelimiter(e.target.value as any)}
                className="bg-transparent text-xs text-foreground font-mono focus:outline-none cursor-pointer"
              >
                <option value=" ">Space (4d 49)</option>
                <option value="">None (4d49)</option>
                <option value=":">Colon (4d:49)</option>
                <option value="\x">\x Prefix (\x4d\x49)</option>
                <option value="0x">0x Prefix (0x4d 0x49)</option>
              </select>

              <button
                onClick={() => setHexCase(hexCase === 'lower' ? 'upper' : 'lower')}
                className="ml-1 px-1.5 py-0.5 rounded text-2xs font-mono font-bold bg-neutral-subtle text-foreground hover:bg-surface border border-border"
                title="Toggle Hex Case"
              >
                {hexCase === 'upper' ? 'UPPER' : 'lower'}
              </button>
            </div>
          )}

          {algorithm === 'url' && direction === 'encode' && (
            <div className="flex items-center gap-1.5 bg-background border border-border rounded px-2 py-1">
              <span className="text-2xs text-muted-foreground font-medium">Scope:</span>
              <button
                onClick={() => setUrlFull(false)}
                className={`px-1.5 py-0.5 rounded text-2xs font-medium ${
                  !urlFull ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground'
                }`}
              >
                Standard
              </button>
              <button
                onClick={() => setUrlFull(true)}
                className={`px-1.5 py-0.5 rounded text-2xs font-medium ${
                  urlFull ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground'
                }`}
                title="Encode every character including alphanumeric"
              >
                All Chars
              </button>
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
            onClick={handleSwap}
            disabled={!outputText || !!errorMsg}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium disabled:opacity-40 disabled:cursor-not-allowed transition-all"
            title="Swap Output into Input (switches encode/decode if bidirectional)"
          >
            <MingCuteIcon name="transfer_line" size={13} />
            <span className="hidden sm:inline">Swap</span>
          </button>

          <button
            onClick={() => setInputText('')}
            disabled={!inputText}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle border border-transparent hover:border-border transition-colors disabled:opacity-40"
            title="Clear Input"
          >
            <MingCuteIcon name="delete_2_line" size={13} />
            <span className="hidden sm:inline">Clear</span>
          </button>
        </div>
      </div>

      {/* Main Dual-Pane Area: INPUT (50%) vs OUTPUT (50%) */}
      <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-2 gap-3 overflow-hidden">
        {/* ================= LEFT PANE: INPUT ================= */}
        <div className="flex flex-col bg-surface border border-border rounded-lg overflow-hidden shadow-2xs">
          <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0">
            <div className="flex items-center gap-2">
              <span className="px-2 py-0.5 rounded bg-primary text-primary-foreground font-mono font-bold text-2xs tracking-wider">
                INPUT
              </span>
              <span className="text-muted-foreground font-mono text-2xs">
                {getStats(inputText)}
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <button
                onClick={handleLoadSample}
                className="px-2 py-1 rounded text-2xs font-medium text-muted-foreground hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors"
                title="Load sample test string"
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
              placeholder="Enter or paste text to encode or decode..."
              spellCheck={false}
              autoCapitalize="none"
              autoComplete="off"
              className="w-full h-full bg-transparent text-foreground font-mono text-xs resize-none focus:outline-none p-1 leading-relaxed selection:bg-primary/20"
            />
          </div>
        </div>

        {/* ================= RIGHT PANE: OUTPUT ================= */}
        <div className="flex flex-col bg-surface border border-border rounded-lg overflow-hidden shadow-2xs">
          <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between gap-2 shrink-0">
            <div className="flex items-center gap-2">
              <span className="px-2 py-0.5 rounded bg-neutral-subtle border border-border text-foreground font-mono font-bold text-2xs tracking-wider">
                OUTPUT
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
                {errorMsg
                  ? 'Error'
                  : `${selectedAlgoConfig.name} ${direction === 'encode' ? 'Encoded' : 'Decoded'}`}
              </span>
              {outputText && !errorMsg && (
                <span className="text-muted-foreground font-mono text-2xs hidden sm:inline">
                  {getStats(outputText)}
                </span>
              )}
            </div>

            <div className="flex items-center gap-1.5">
              <button
                onClick={handleUseAsInput}
                disabled={!outputText || !!errorMsg}
                className="px-2 py-1 rounded text-2xs font-medium text-muted-foreground hover:text-foreground hover:bg-neutral-subtle flex items-center gap-1 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                title="Send Output to Input"
              >
                <MingCuteIcon name="transfer_line" size={12} />
                <span>Use as Input</span>
              </button>

              <button
                onClick={() => copyWithFeedback(outputText, 'output')}
                disabled={!outputText || !!errorMsg}
                className={`px-2.5 py-1 rounded text-2xs font-semibold flex items-center gap-1 transition-all ${
                  copiedKey === 'output'
                    ? 'bg-emerald-600 text-white shadow-2xs'
                    : 'bg-primary text-primary-foreground hover:bg-primary-hover shadow-2xs disabled:opacity-40 disabled:cursor-not-allowed'
                }`}
                title="Copy output to clipboard"
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
                <h4 className="font-bold text-foreground text-xs mb-1">Conversion Failed</h4>
                <p className="text-rose-500 font-mono text-xs max-w-md break-all leading-relaxed bg-rose-500/5 p-3 rounded border border-rose-500/20 mb-2">
                  {errorMsg}
                </p>
                <p className="text-muted-foreground text-2xs">
                  Verify the input format matches the expected syntax for{' '}
                  <span className="font-semibold text-foreground">
                    {selectedAlgoConfig.name} {direction}
                  </span>
                  .
                </p>
              </div>
            ) : (
              <textarea
                readOnly
                value={outputText}
                placeholder="Transformed output will appear here automatically..."
                className="w-full h-full bg-transparent text-foreground font-mono text-xs resize-none focus:outline-none p-1 leading-relaxed selection:bg-primary/20 select-all cursor-text"
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
