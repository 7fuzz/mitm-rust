import React, { useState, useRef, useEffect } from 'react';
import type { RepeaterTab } from '../../../services/tauri/bridge';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { MultipartEditor } from '../../common/MultipartEditor';
import { UrlEncodedEditor } from '../../common/UrlEncodedEditor';
import { ExtractRulesEditor } from '../../common/ExtractRulesEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';

interface RequestBuilderProps {
  request: RepeaterTab;
}

const METHOD_OPTIONS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
  { value: 'HEAD', label: 'HEAD' },
] as const;

export const RequestBuilder: React.FC<RequestBuilderProps> = ({ request }) => {
  const { updateTab, executeActiveRequest, isExecuting, toggleHistoryDrawer } = useRepeaterStore();
  const [activeTab, setActiveTab] = useState<'params' | 'headers' | 'body' | 'auto-extract'>('body');
  const [copyMenuOpen, setCopyMenuOpen] = useState(false);
  const [copyNotification, setCopyNotification] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const executing = isExecuting[request.id] || false;

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setCopyMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const showNotification = (msg: string) => {
    setCopyNotification(msg);
    setCopyMenuOpen(false);
    setTimeout(() => setCopyNotification(null), 1800);
  };

  const handleCopyBody = () => {
    const body = request.bodyContent || '';
    navigator.clipboard.writeText(body);
    showNotification('Copied Body!');
  };

  const handleCopyHeaders = () => {
    const rawHeaders = (request.headers || [])
      .filter((h) => h.enabled && h.key.trim())
      .map((h) => `${h.key}: ${h.value}`)
      .join('\n');
    navigator.clipboard.writeText(rawHeaders);
    showNotification('Copied Headers!');
  };

  const handleCopyAll = () => {
    const rawHeaders = (request.headers || [])
      .filter((h) => h.enabled && h.key.trim())
      .map((h) => `${h.key}: ${h.value}`)
      .join('\n');
    const fullReq = `${request.method} ${request.url}\n${rawHeaders}${request.bodyContent ? `\n\n${request.bodyContent}` : ''}`;
    navigator.clipboard.writeText(fullReq);
    showNotification('Copied Full Request!');
  };

  const handleCopyCurl = () => {
    const headersStr = (request.headers || [])
      .filter((h) => h.enabled && h.key.trim())
      .map((h) => `-H '${h.key}: ${h.value}'`)
      .join(' ');
    const bodyStr = request.bodyContent ? `-d '${request.bodyContent.replace(/'/g, "'\\''")}'` : '';
    const curlCmd = `curl -X ${request.method} '${request.url}' ${headersStr} ${bodyStr}`.trim();
    navigator.clipboard.writeText(curlCmd);
    showNotification('Copied as cURL!');
  };

  const handleMethodChange = (method: string) => {
    updateTab({ ...request, method });
  };

  const handleUrlChange = (url: string) => {
    updateTab({ ...request, url });
  };

  const handleBodyTypeChange = (newBodyType: string) => {
    let headers = [...(request.headers || [])];
    const ctIdx = headers.findIndex((h) => h.key.trim().toLowerCase() === 'content-type');

    if (newBodyType === 'none') {
      if (ctIdx >= 0) {
        headers[ctIdx] = { ...headers[ctIdx], enabled: false };
      }
    } else {
      let targetCt = 'application/json';
      if (newBodyType === 'json') targetCt = 'application/json';
      else if (newBodyType === 'urlencoded' || newBodyType === 'x-www-form-urlencoded') targetCt = 'application/x-www-form-urlencoded';
      else if (newBodyType === 'form' || newBodyType === 'form-data' || newBodyType === 'multipart') targetCt = 'multipart/form-data';
      else if (newBodyType === 'raw') targetCt = 'text/plain';

      if (ctIdx >= 0) {
        headers[ctIdx] = {
          ...headers[ctIdx],
          value: targetCt,
          enabled: true,
        };
      } else {
        headers.push({
          id: `h-ct-${Date.now()}`,
          key: 'Content-Type',
          value: targetCt,
          enabled: true,
        });
      }
    }

    updateTab({ ...request, bodyType: newBodyType, headers });
  };

  const handleAddWebHeaders = () => {
    const currentHeaders = [...(request.headers || [])];
    const standardHeaders = [
      { key: 'User-Agent', value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36', enabled: true },
      { key: 'Accept', value: '*/*', enabled: true },
      { key: 'Accept-Language', value: 'en-US,en;q=0.9', enabled: true },
      { key: 'Accept-Encoding', value: 'gzip, deflate, br, zstd', enabled: true },
      { key: 'Connection', value: 'keep-alive', enabled: true },
      { key: 'Sec-Fetch-Dest', value: 'empty', enabled: true },
      { key: 'Sec-Fetch-Mode', value: 'cors', enabled: true },
      { key: 'Sec-Fetch-Site', value: 'same-site', enabled: true },
      {
        key: 'Content-Type',
        value: request.bodyType === 'json'
          ? 'application/json'
          : request.bodyType === 'urlencoded'
          ? 'application/x-www-form-urlencoded'
          : request.bodyType === 'form'
          ? 'multipart/form-data'
          : request.bodyType === 'raw'
          ? 'text/plain'
          : 'application/json',
        enabled: request.bodyType !== 'none',
      },
    ];

    let hasAdded = false;
    for (const sh of standardHeaders) {
      const exists = currentHeaders.some((h) => h.key.trim().toLowerCase() === sh.key.toLowerCase());
      if (!exists) {
        currentHeaders.push({
          id: `h-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
          key: sh.key,
          value: sh.value,
          enabled: sh.enabled,
        });
        hasAdded = true;
      }
    }

    if (hasAdded) {
      updateTab({ ...request, headers: currentHeaders });
      showNotification('Added Web Headers!');
    } else {
      showNotification('Headers already present');
    }
  };

  const handleExecute = async () => {
    await executeActiveRequest(request.id);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleExecute();
    }
  };

  return (
    <div className="flex-1 flex flex-col bg-surface border-r border-border overflow-hidden text-xs">
      {/* Top URL Bar */}
      <div className="p-3 bg-header border-b border-border flex items-center gap-2 shrink-0">
        {/* Method Select */}
        <Select
          value={request.method}
          onChange={(e) => handleMethodChange(e.target.value)}
          options={METHOD_OPTIONS}
          sizeVariant="sm"
        />

        {/* URL Input */}
        <div className="flex-1 relative flex items-center">
          <input
            type="text"
            value={request.url}
            onChange={(e) => handleUrlChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="https://api.example.com/v1/resource or {{BASE_URL}}/path"
            className="w-full bg-background border border-border rounded px-3 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
          />
        </div>

        {/* Send Action Button */}
        <button
          disabled={executing}
          onClick={handleExecute}
          className="flex items-center gap-1.5 px-4 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover transition-colors shadow-xs disabled:opacity-50 cursor-pointer"
        >
          <MingCuteIcon name="send_plane_line" size={15} />
          <span>{executing ? 'Sending...' : 'Send (⌘↵)'}</span>
        </button>

        {/* History Drawer Trigger */}
        <button
          onClick={() => toggleHistoryDrawer()}
          className="p-1.5 rounded bg-background border border-border text-muted-foreground hover:text-foreground cursor-pointer"
          title="View Execution History"
        >
          <MingCuteIcon name="history_line" size={16} />
        </button>
      </div>

      {/* Request Config Tabs & Copy Dropdown */}
      <div className="bg-header border-b border-border px-3 py-1 flex items-center justify-between shrink-0 select-none">
        <div className="flex items-center gap-2">
          {(['params', 'headers', 'body', 'auto-extract'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-3 py-1 rounded text-xs font-medium uppercase transition-colors cursor-pointer ${
                activeTab === tab
                  ? 'bg-surface text-primary border border-border/80 shadow-2xs font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab === 'auto-extract' ? 'Auto-Extract' : tab}
            </button>
          ))}
        </div>

        {/* Copy Request Actions Dropdown */}
        <div className="relative" ref={menuRef}>
          <button
            onClick={() => setCopyMenuOpen(!copyMenuOpen)}
            className="flex items-center gap-1 px-2.5 py-1 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs font-medium"
            title="Copy Request Options"
          >
            <MingCuteIcon name={copyNotification ? 'check_line' : 'copy_line'} size={13} className={copyNotification ? 'text-emerald-400' : ''} />
            <span>{copyNotification || 'Copy Request'}</span>
            <MingCuteIcon name="down_line" size={12} className="opacity-60" />
          </button>

          {copyMenuOpen && (
            <div className="absolute right-0 mt-1 w-44 bg-surface border border-border rounded-lg shadow-xl py-1 z-50 font-mono text-xs flex flex-col">
              <button
                onClick={handleCopyBody}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer"
              >
                <MingCuteIcon name="file_text_line" size={14} className="text-primary" />
                <span>Copy Body</span>
              </button>
              <button
                onClick={handleCopyHeaders}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer"
              >
                <MingCuteIcon name="list_check_line" size={14} className="text-emerald-500" />
                <span>Copy Headers</span>
              </button>
              <button
                onClick={handleCopyAll}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer border-t border-border/80"
              >
                <MingCuteIcon name="copy_line" size={14} className="text-amber-500" />
                <span>Copy All (Full Req)</span>
              </button>
              <button
                onClick={handleCopyCurl}
                className="px-3 py-1.5 text-left text-foreground hover:bg-neutral-subtle flex items-center gap-2 transition-colors cursor-pointer"
              >
                <MingCuteIcon name="code_line" size={14} className="text-blue-500" />
                <span>Copy as cURL</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Tab Panels */}
      <div className="flex-1 p-3 overflow-auto">
        {activeTab === 'params' && (
          <KeyValueEditor
            items={request.params || []}
            onChange={(params) => updateTab({ ...request, params })}
            keyPlaceholder="Parameter Key"
            valuePlaceholder="Parameter Value"
          />
        )}

        {activeTab === 'headers' && (
          <div className="space-y-2">
            <div className="flex items-center justify-between pb-1 text-xs">
              <div className="flex items-center gap-1.5 text-muted-foreground text-[11px]">
                <MingCuteIcon name="alert_line" size={13} className="text-primary shrink-0" />
                <span>
                  <strong>Tip:</strong> If <code>Content-Type</code> is disabled, it will be <strong>auto-detected</strong> from the body format.
                </span>
              </div>
              <button
                type="button"
                onClick={handleAddWebHeaders}
                className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-primary font-medium text-[11px] cursor-pointer transition-colors shrink-0"
                title="Add missing standard web headers (User-Agent, Accept, Encoding, etc.)"
              >
                <MingCuteIcon name="plus_line" size={12} />
                <span>+ Add Web Headers</span>
              </button>
            </div>
            <KeyValueEditor
              items={request.headers || []}
              onChange={(headers) => updateTab({ ...request, headers })}
              keyPlaceholder="Header Name (e.g. Authorization)"
              valuePlaceholder="Header Value (e.g. Bearer token)"
            />
          </div>
        )}

        {activeTab === 'body' && (
          <div className="h-full flex flex-col overflow-hidden">
            <div className="flex items-center gap-2 mb-2">
              <Select
                value={request.bodyType}
                onChange={(e) => handleBodyTypeChange(e.target.value)}
                options={[
                  { value: 'none', label: 'None' },
                  { value: 'json', label: 'JSON' },
                  { value: 'raw', label: 'Raw Text' },
                  { value: 'form', label: 'Form Data (Multipart)' },
                  { value: 'urlencoded', label: 'URL-Encoded (x-www-form-urlencoded)' },
                ]}
                sizeVariant="xs"
              />
            </div>
            {request.bodyType === 'urlencoded' || request.bodyType === 'x-www-form-urlencoded' ? (
              <div className="flex-1 overflow-y-auto">
                <UrlEncodedEditor
                  params={(() => {
                    try {
                      const parsed = JSON.parse(request.bodyContent || '');
                      if (Array.isArray(parsed)) return parsed;
                    } catch {}
                    return [];
                  })()}
                  onChange={(newParams) => {
                    const jsonStr = JSON.stringify(newParams, null, 2);
                    updateTab({ ...request, bodyContent: jsonStr });
                  }}
                />
              </div>
            ) : request.bodyType === 'form' || request.bodyType === 'form-data' || request.bodyType === 'multipart' ? (
              <div className="flex-1 overflow-y-auto">
                <MultipartEditor
                  fields={(() => {
                    try {
                      const parsed = JSON.parse(request.bodyContent || '');
                      if (parsed && Array.isArray(parsed.__form_data)) return parsed.__form_data;
                      if (Array.isArray(parsed)) return parsed;
                    } catch {}
                    return [];
                  })()}
                  onChange={(fields) => {
                    const jsonStr = JSON.stringify({ __form_data: fields }, null, 2);
                    updateTab({ ...request, bodyContent: jsonStr });
                  }}
                />
              </div>
            ) : request.bodyType !== 'none' ? (
              <div className="flex-1 overflow-hidden border border-border rounded">
                <CodeEditor
                  value={request.bodyContent || ''}
                  onChange={(bodyContent) => updateTab({ ...request, bodyContent })}
                  language={request.bodyType === 'json' ? 'json' : 'plaintext'}
                />
              </div>
            ) : null}
          </div>
        )}

        {activeTab === 'auto-extract' && (
          <ExtractRulesEditor
            rules={request.extractRules || []}
            onChange={(extractRules) => updateTab({ ...request, extractRules })}
          />
        )}
      </div>
    </div>
  );
};
