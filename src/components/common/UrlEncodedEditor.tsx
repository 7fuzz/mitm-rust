import React, { useState } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import type { UrlEncodedParam } from '../../types';

interface UrlEncodedEditorProps {
  params: UrlEncodedParam[];
  onChange: (params: UrlEncodedParam[]) => void;
  readOnly?: boolean;
}

export const UrlEncodedEditor: React.FC<UrlEncodedEditorProps> = ({
  params,
  onChange,
  readOnly = false,
}) => {
  const [showBulkPaste, setShowBulkPaste] = useState(false);
  const [bulkRawText, setBulkRawText] = useState('');

  const handleAddParam = () => {
    const newParam: UrlEncodedParam = {
      id: 'urlp_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
      enabled: true,
      key: '',
      value: '',
    };
    onChange([...params, newParam]);
  };

  const handleParamChange = (index: number, patch: Partial<UrlEncodedParam>) => {
    const updated = [...params];
    updated[index] = { ...updated[index], ...patch };
    onChange(updated);
  };

  const handleDeleteParam = (index: number) => {
    const updated = params.filter((_, i) => i !== index);
    onChange(updated);
  };

  const handleBulkParse = () => {
    if (!bulkRawText.trim()) return;

    // Parse query string like "key1=val1&key2=val2" or line-separated "key1=val1\nkey2=val2"
    const pairs: UrlEncodedParam[] = [];
    const textToParse = bulkRawText.replace(/\n/g, '&');
    const searchParams = new URLSearchParams(textToParse);

    searchParams.forEach((val, key) => {
      pairs.push({
        id: 'urlp_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
        enabled: true,
        key: key.trim(),
        value: val,
      });
    });

    if (pairs.length > 0) {
      onChange([...params, ...pairs]);
      setBulkRawText('');
      setShowBulkPaste(false);
    }
  };

  return (
    <div className="w-full flex flex-col gap-2 font-mono text-xs select-none">
      {/* Bulk Converter Modal / Panel */}
      {showBulkPaste && (
        <div className="bg-header border border-border rounded-lg p-3 space-y-2 mb-1">
          <div className="flex items-center justify-between font-sans">
            <span className="font-semibold text-foreground flex items-center gap-1.5">
              <MingCuteIcon name="lightning_line" size={15} className="text-amber-500" />
              <span>Bulk Convert Raw URL-Encoded String</span>
            </span>
            <button
              onClick={() => setShowBulkPaste(false)}
              className="text-muted-foreground hover:text-foreground p-0.5 rounded"
            >
              <MingCuteIcon name="close_line" size={14} />
            </button>
          </div>

          <textarea
            value={bulkRawText}
            onChange={(e) => setBulkRawText(e.target.value)}
            placeholder="Paste raw string, e.g. username=admin&token=xyz123&grant_type=password"
            rows={3}
            className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary resize-none"
          />

          <div className="flex items-center justify-end gap-2 font-sans">
            <button
              onClick={() => setShowBulkPaste(false)}
              className="px-2.5 py-1 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground text-xs"
            >
              Cancel
            </button>
            <button
              onClick={handleBulkParse}
              disabled={!bulkRawText.trim()}
              className="px-3 py-1 rounded bg-primary text-primary-foreground font-semibold text-xs cursor-pointer disabled:opacity-40"
            >
              Parse & Append
            </button>
          </div>
        </div>
      )}

      {/* URL-Encoded Table */}
      <div className="border border-border rounded-lg overflow-hidden">
        <table className="w-full text-left">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-2xs">
              <th className="w-8 px-2 py-1.5 text-center">En</th>
              <th className="w-1/3 px-3 py-1.5 font-medium">Parameter Key</th>
              <th className="px-3 py-1.5 font-medium">Value</th>
              {!readOnly && <th className="w-8 px-2 py-1.5 text-center"></th>}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {params.length === 0 ? (
              <tr>
                <td colSpan={readOnly ? 3 : 4} className="px-3 py-6 text-center text-muted-foreground italic font-sans text-xs">
                  No URL-encoded parameters added.
                </td>
              </tr>
            ) : (
              params.map((param, idx) => (
                <tr key={param.id || idx} className="hover:bg-neutral-subtle/50">
                  {/* Enable Checkbox */}
                  <td className="px-2 py-1 text-center">
                    <input
                      type="checkbox"
                      checked={param.enabled}
                      disabled={readOnly}
                      onChange={(e) => handleParamChange(idx, { enabled: e.target.checked })}
                      className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer accent-primary"
                    />
                  </td>

                  {/* Key Input */}
                  <td className="px-2 py-1">
                    <input
                      type="text"
                      placeholder="key_name"
                      value={param.key}
                      disabled={readOnly}
                      onChange={(e) => handleParamChange(idx, { key: e.target.value })}
                      className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
                    />
                  </td>

                  {/* Value Input */}
                  <td className="px-2 py-1">
                    <input
                      type="text"
                      placeholder="param_value"
                      value={param.value}
                      disabled={readOnly}
                      onChange={(e) => handleParamChange(idx, { value: e.target.value })}
                      className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
                    />
                  </td>

                  {/* Delete Row Button */}
                  {!readOnly && (
                    <td className="px-2 py-1 text-center">
                      <button
                        type="button"
                        onClick={() => handleDeleteParam(idx)}
                        className="text-muted-foreground hover:text-rose-500 p-1 rounded transition-colors cursor-pointer"
                        title="Remove Parameter"
                      >
                        <MingCuteIcon name="close_line" size={13} />
                      </button>
                    </td>
                  )}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {!readOnly && (
        <div className="flex items-center justify-between pt-0.5 font-sans">
          <button
            type="button"
            onClick={handleAddParam}
            className="px-3 py-1 rounded bg-background hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs cursor-pointer transition-colors flex items-center gap-1"
          >
            <MingCuteIcon name="plus_line" size={13} />
            <span>Add Parameter</span>
          </button>

          <button
            type="button"
            onClick={() => setShowBulkPaste(!showBulkPaste)}
            className="px-2.5 py-1 rounded bg-header hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground text-xs cursor-pointer transition-colors flex items-center gap-1"
          >
            <MingCuteIcon name="lightning_line" size={13} className="text-amber-500" />
            <span>Bulk Convert Raw String</span>
          </button>
        </div>
      )}
    </div>
  );
};
