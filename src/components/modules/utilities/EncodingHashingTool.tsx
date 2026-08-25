import React, { useState, useMemo } from 'react';
import CryptoJS from 'crypto-js';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const EncodingHashingTool: React.FC = () => {
  const [inputText, setInputText] = useState('MITM-Developer-Security-Suite-2.0');

  const conversions = useMemo(() => {
    if (!inputText) return [];

    try {
      const base64Enc = btoa(unescape(encodeURIComponent(inputText)));
      let base64Dec = '';
      try {
        base64Dec = decodeURIComponent(escape(atob(inputText)));
      } catch (e) {
        base64Dec = '<Invalid Base64 Input>';
      }

      const urlEnc = encodeURIComponent(inputText);
      const urlDec = decodeURI(inputText);

      const hexEnc = Array.from(new TextEncoder().encode(inputText))
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');

      const md5Hash = CryptoJS.MD5(inputText).toString();
      const sha256Hash = CryptoJS.SHA256(inputText).toString();
      const sha512Hash = CryptoJS.SHA512(inputText).toString();

      return [
        { label: 'Base64 Encoded', val: base64Enc },
        { label: 'Base64 Decoded', val: base64Dec },
        { label: 'URL Encoded', val: urlEnc },
        { label: 'URL Decoded', val: urlDec },
        { label: 'Hex String', val: hexEnc },
        { label: 'MD5 Hash', val: md5Hash },
        { label: 'SHA-256 Hash', val: sha256Hash },
        { label: 'SHA-512 Hash', val: sha512Hash },
      ];
    } catch (err) {
      return [];
    }
  }, [inputText]);

  const copyToClipboard = (val: string) => {
    navigator.clipboard.writeText(val);
  };

  return (
    <div className="h-full flex flex-col gap-4 text-xs overflow-y-auto">
      {/* Input Area */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-2 shrink-0">
        <label className="font-semibold text-foreground text-sm block">Input Raw String:</label>
        <textarea
          rows={3}
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
          placeholder="Enter text to convert into Base64, URL Encoding, Hex, MD5, SHA-256..."
          className="w-full bg-background border border-border rounded p-2.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
        />
      </div>

      {/* Output Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {conversions.map((item, idx) => (
          <div key={idx} className="bg-surface border border-border rounded-lg p-3 space-y-1.5 font-mono">
            <div className="flex items-center justify-between">
              <span className="font-bold text-primary font-sans text-xs">{item.label}</span>
              <button
                onClick={() => copyToClipboard(item.val)}
                className="text-muted-foreground hover:text-foreground p-1 rounded hover:bg-neutral-subtle"
                title="Copy output"
              >
                <MingCuteIcon name="copy_line" size={13} />
              </button>
            </div>
            <div className="p-2 bg-background border border-border rounded text-foreground font-mono text-xs break-all select-all">
              {item.val || '<empty>'}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
