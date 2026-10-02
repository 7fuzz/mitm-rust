import React from 'react';
import type { KeyValuePair } from '../../types';
import { MingCuteIcon } from './MingCuteIcon';
import { StandardHeaderPicker, type StandardHeader } from './StandardHeaderPicker';

interface KeyValueEditorProps {
  items: KeyValuePair[];
  onChange: (items: KeyValuePair[]) => void;
  readOnly?: boolean;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  className?: string;
  /** When set, shows an "Add Standard Header" picker next to Add Row */
  standardHeaders?: StandardHeader[];
}

const COMMON_HEADER_KEYS = [
  'Accept',
  'Accept-Charset',
  'Accept-Encoding',
  'Accept-Language',
  'Authorization',
  'Cache-Control',
  'Connection',
  'Content-Disposition',
  'Content-Encoding',
  'Content-Length',
  'Content-Type',
  'Cookie',
  'Host',
  'If-Match',
  'If-Modified-Since',
  'If-None-Match',
  'Origin',
  'Pragma',
  'Range',
  'Referer',
  'Sec-Ch-Ua',
  'Sec-Ch-Ua-Mobile',
  'Sec-Ch-Ua-Platform',
  'Sec-Fetch-Dest',
  'Sec-Fetch-Mode',
  'Sec-Fetch-Site',
  'Sec-Fetch-User',
  'Upgrade-Insecure-Requests',
  'User-Agent',
  'X-Forwarded-For',
  'X-Requested-With',
];

const COMMON_HEADER_VALUES = [
  'application/json',
  'application/x-www-form-urlencoded',
  'multipart/form-data',
  'text/plain',
  'text/html; charset=utf-8',
  'application/xml',
  'application/octet-stream',
  '*/*',
  'application/json, text/plain, */*',
  'gzip, deflate, br, zstd',
  'en-US,en;q=0.9',
  'keep-alive',
  'no-cache',
  'cors',
  'empty',
  'same-site',
  'same-origin',
  'cross-site',
];

export const KeyValueEditor: React.FC<KeyValueEditorProps> = ({
  items,
  onChange,
  readOnly = false,
  keyPlaceholder = 'Key',
  valuePlaceholder = 'Value',
  className = '',
  standardHeaders,
}) => {
  const isHeaderEditor = keyPlaceholder.toLowerCase().includes('header');
  const datalistKeyId = isHeaderEditor ? 'http-header-keys-list' : undefined;
  const datalistValId = isHeaderEditor ? 'http-header-vals-list' : undefined;

  const handleItemChange = (index: number, field: keyof KeyValuePair, value: any) => {
    const updated = [...items];
    updated[index] = { ...updated[index], [field]: value };
    onChange(updated);
  };

  const handleToggleEnable = (index: number) => {
    const updated = [...items];
    updated[index].enabled = !updated[index].enabled;
    onChange(updated);
  };

  const handleDelete = (index: number) => {
    const updated = items.filter((_, i) => i !== index);
    onChange(updated);
  };

  const handleAddRow = (key = '', value = '') => {
    const newItem: KeyValuePair = {
      id: 'kv-' + Date.now() + '-' + Math.random().toString(36).substring(2, 5),
      key,
      value,
      enabled: true,
    };
    onChange([...items, newItem]);
  };

  return (
    <div className={`space-y-2 text-xs ${className}`}>
      {isHeaderEditor && (
        <>
          <datalist id="http-header-keys-list">
            {COMMON_HEADER_KEYS.map((k) => (
              <option key={k} value={k} />
            ))}
          </datalist>
          <datalist id="http-header-vals-list">
            {COMMON_HEADER_VALUES.map((v) => (
              <option key={v} value={v} />
            ))}
          </datalist>
        </>
      )}

      <div className="border border-border rounded overflow-hidden">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-2xs font-medium">
              {!readOnly && <th className="w-8 px-2 py-1.5 text-center">#</th>}
              <th className="px-3 py-1.5 font-medium">{keyPlaceholder}</th>
              <th className="px-3 py-1.5 font-medium">{valuePlaceholder}</th>
              {!readOnly && <th className="w-10 px-2 py-1.5 text-center">Actions</th>}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {items.length === 0 ? (
              <tr>
                <td colSpan={readOnly ? 2 : 4} className="px-3 py-3 text-center text-muted-foreground italic">
                  No items configured
                </td>
              </tr>
            ) : (
              items.map((item, index) => (
                <tr key={item.id || index} className="hover:bg-neutral-subtle/50 transition-colors">
                  {!readOnly && (
                    <td className="px-2 py-1 text-center">
                      <input
                        type="checkbox"
                        checked={item.enabled}
                        onChange={() => handleToggleEnable(index)}
                        className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer"
                      />
                    </td>
                  )}
                  <td className="px-2 py-1">
                    {readOnly ? (
                      <span className={`font-mono ${!item.enabled ? 'line-through text-muted-foreground' : 'text-foreground'}`}>
                        {item.key}
                      </span>
                    ) : (
                      <input
                        type="text"
                        list={datalistKeyId}
                        value={item.key}
                        onChange={(e) => handleItemChange(index, 'key', e.target.value)}
                        placeholder={keyPlaceholder}
                        className={`w-full bg-transparent px-1.5 py-1 font-mono text-xs focus:outline-none border border-transparent focus:border-border rounded ${
                          !item.enabled ? 'line-through text-muted-foreground' : 'text-foreground'
                        }`}
                      />
                    )}
                  </td>
                  <td className="px-2 py-1">
                    {readOnly ? (
                      <span className={`font-mono break-all ${!item.enabled ? 'line-through text-muted-foreground' : 'text-foreground'}`}>
                        {item.value}
                      </span>
                    ) : (
                      <input
                        type="text"
                        list={datalistValId}
                        value={item.value}
                        onChange={(e) => handleItemChange(index, 'value', e.target.value)}
                        placeholder={valuePlaceholder}
                        className={`w-full bg-transparent px-1.5 py-1 font-mono text-xs focus:outline-none border border-transparent focus:border-border rounded ${
                          !item.enabled ? 'line-through text-muted-foreground' : 'text-foreground'
                        }`}
                      />
                    )}
                  </td>
                  {!readOnly && (
                    <td className="px-2 py-1 text-center">
                      <button
                        onClick={() => handleDelete(index)}
                        className="p-1 text-muted-foreground hover:text-rose-500 rounded transition-colors"
                        title="Delete row"
                      >
                        <MingCuteIcon name="delete_2_line" size={14} />
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
        <div className="flex items-center gap-2">
          <button
            onClick={() => handleAddRow()}
            className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-neutral-subtle border border-border text-foreground hover:bg-neutral-subtle/80 text-xs font-medium transition-colors"
          >
            <MingCuteIcon name="plus_line" size={14} />
            Add Row
          </button>
          {standardHeaders && (
            <StandardHeaderPicker
              headers={standardHeaders}
              existingKeys={items.map((i) => i.key)}
              onPick={(h) => handleAddRow(h.key, h.value)}
            />
          )}
        </div>
      )}
    </div>
  );
};
