import React, { useState } from 'react';
import { CodeEditor } from '../../common/CodeEditor';

export const JsonToolkit: React.FC = () => {
  const [inputJson, setInputJson] = useState(
    JSON.stringify({ user: 'admin', roles: ['security_admin', 'auditor'], metadata: { active: true, login_attempts: 0 } })
  );
  const [outputJson, setOutputJson] = useState('');
  const [statusMsg, setStatusMsg] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  const handleFormat = () => {
    try {
      const parsed = JSON.parse(inputJson);
      const formatted = JSON.stringify(parsed, null, 2);
      setOutputJson(formatted);
      setStatusMsg({ text: 'Valid JSON formatted successfully', type: 'success' });
    } catch (e: any) {
      setStatusMsg({ text: `Invalid JSON: ${e.message}`, type: 'error' });
    }
  };

  const handleMinify = () => {
    try {
      const parsed = JSON.parse(inputJson);
      const minified = JSON.stringify(parsed);
      setOutputJson(minified);
      setStatusMsg({ text: 'Valid JSON minified successfully', type: 'success' });
    } catch (e: any) {
      setStatusMsg({ text: `Invalid JSON: ${e.message}`, type: 'error' });
    }
  };

  const handleEscape = () => {
    const escaped = JSON.stringify(inputJson);
    setOutputJson(escaped);
    setStatusMsg({ text: 'String escaped', type: 'success' });
  };

  const handleUnescape = () => {
    try {
      const unescaped = JSON.parse(inputJson);
      setOutputJson(typeof unescaped === 'string' ? unescaped : JSON.stringify(unescaped, null, 2));
      setStatusMsg({ text: 'String unescaped', type: 'success' });
    } catch (e: any) {
      setStatusMsg({ text: 'Failed to unescape string', type: 'error' });
    }
  };

  return (
    <div className="h-full flex flex-col gap-3 text-xs overflow-hidden">
      {/* Control Action Toolbar */}
      <div className="p-3 bg-surface border border-border rounded-lg flex items-center justify-between gap-3 shrink-0">
        <div className="flex items-center gap-2">
          <button
            onClick={handleFormat}
            className="px-3 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover transition-colors shadow-xs"
          >
            Format Pretty
          </button>
          <button
            onClick={handleMinify}
            className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
          >
            Minify
          </button>
          <button
            onClick={handleEscape}
            className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
          >
            Escape String
          </button>
          <button
            onClick={handleUnescape}
            className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
          >
            Unescape String
          </button>
        </div>

        {statusMsg && (
          <span
            className={`font-mono text-xs font-semibold ${
              statusMsg.type === 'success' ? 'text-emerald-500' : 'text-rose-500'
            }`}
          >
            {statusMsg.text}
          </span>
        )}
      </div>

      {/* Dual Pane Editors: Left Input (50%) vs Right Output (50%) */}
      <div className="flex-1 grid grid-cols-1 md:grid-cols-2 gap-3 overflow-hidden">
        <div className="flex flex-col gap-1 overflow-hidden bg-surface border border-border rounded-lg p-2">
          <span className="font-semibold text-foreground text-2xs uppercase tracking-wider">Input JSON:</span>
          <div className="flex-1 overflow-hidden">
            <CodeEditor value={inputJson} onChange={setInputJson} language="json" readOnly={false} />
          </div>
        </div>

        <div className="flex flex-col gap-1 overflow-hidden bg-surface border border-border rounded-lg p-2">
          <span className="font-semibold text-foreground text-2xs uppercase tracking-wider">Transformed Output:</span>
          <div className="flex-1 overflow-hidden">
            <CodeEditor value={outputJson} language="json" readOnly />
          </div>
        </div>
      </div>
    </div>
  );
};
