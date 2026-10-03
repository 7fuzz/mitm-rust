import React, { useState, useEffect } from 'react';
import type { RepeaterTab, ParamItem } from '../../../services/tauri/bridge';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { MultipartEditor } from '../../common/MultipartEditor';
import { UrlEncodedEditor } from '../../common/UrlEncodedEditor';
import { ExtractRulesEditor } from '../../common/ExtractRulesEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';
import { PaneHeader } from '../../common/PaneHeader';
import { buildUrlWithParams, parseUrlQueryParams } from '../../../utils/urlParams';
import { tryPrettifyJson } from '../../../utils/prettifyJson';

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

// Short labels: a native select is as wide as its longest option, and it sits in the tab row
const BODY_TYPE_OPTIONS = [
  { value: 'none', label: 'None' },
  { value: 'json', label: 'JSON' },
  { value: 'raw', label: 'Raw' },
  { value: 'form', label: 'Multipart' },
  { value: 'urlencoded', label: 'URL-Encoded' },
];

/** Enabled rows with a key; what actually gets sent */
const countActive = (items?: Array<{ key: string; enabled: boolean }>) =>
  (items || []).filter((i) => i.enabled && i.key.trim()).length;

export const RequestBuilder: React.FC<RequestBuilderProps> = ({ request }) => {
  const { updateTab, executeActiveRequest, isExecuting, toggleHistoryDrawer } = useRepeaterStore();
  const [activeTab, setActiveTab] = useState<'params' | 'headers' | 'body' | 'auto-extract'>('body');

  const executing = isExecuting[request.id] || false;

  useEffect(() => {
    if (!request) return;
    const hasQuery = request.url.includes('?');
    const activeParams = (request.params || []).filter(
      (p) => p.enabled && (p.key.trim() !== '' || p.value.trim() !== '')
    );
    if (!hasQuery && activeParams.length > 0) {
      const syncedUrl = buildUrlWithParams(request.url, request.params || []);
      updateTab({ ...request, url: syncedUrl });
    } else if (hasQuery && (!request.params || request.params.length === 0)) {
      const syncedParams = parseUrlQueryParams(request.url, []);
      if (syncedParams.length > 0) {
        updateTab({ ...request, params: syncedParams });
      }
    }
  }, [request.id]);

  const handleMethodChange = (method: string) => {
    updateTab({ ...request, method });
  };

  const handleUrlChange = (url: string) => {
    const newParams = parseUrlQueryParams(url, request.params || []);
    updateTab({ ...request, url, params: newParams });
  };

  const handleParamsChange = (newParams: any[]) => {
    const formattedParams: ParamItem[] = newParams.map((p) => ({
      id: p.id || `p-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      key: p.key,
      value: p.value,
      enabled: p.enabled,
    }));
    const newUrl = buildUrlWithParams(request.url, formattedParams);
    updateTab({ ...request, url: newUrl, params: formattedParams });
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

  const contentTypeForBody =
    request.bodyType === 'urlencoded'
      ? 'application/x-www-form-urlencoded'
      : request.bodyType === 'form'
      ? 'multipart/form-data'
      : request.bodyType === 'raw'
      ? 'text/plain'
      : 'application/json';

  const standardHeaders = [
    { key: 'Accept', value: '*/*' },
    { key: 'Accept-Encoding', value: 'gzip, deflate, br, zstd' },
    { key: 'Accept-Language', value: 'en-US,en;q=0.9' },
    { key: 'Authorization', value: 'Bearer ' },
    { key: 'Cache-Control', value: 'no-cache' },
    { key: 'Connection', value: 'keep-alive' },
    { key: 'Content-Type', value: contentTypeForBody },
    { key: 'Cookie', value: '' },
    { key: 'If-Modified-Since', value: '' },
    { key: 'If-None-Match', value: '' },
    { key: 'Origin', value: '' },
    { key: 'Pragma', value: 'no-cache' },
    { key: 'Range', value: 'bytes=0-' },
    { key: 'Referer', value: '' },
    { key: 'Sec-Ch-Ua', value: '"Chromium";v="124", "Google Chrome";v="124", "Not-A.Brand";v="99"' },
    { key: 'Sec-Ch-Ua-Mobile', value: '?0' },
    { key: 'Sec-Ch-Ua-Platform', value: '"Windows"' },
    { key: 'Sec-Fetch-Dest', value: 'empty' },
    { key: 'Sec-Fetch-Mode', value: 'cors' },
    { key: 'Sec-Fetch-Site', value: 'same-site' },
    { key: 'Upgrade-Insecure-Requests', value: '1' },
    { key: 'User-Agent', value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36' },
    { key: 'X-Forwarded-For', value: '127.0.0.1' },
    { key: 'X-Requested-With', value: 'XMLHttpRequest' },
  ];

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
      <div className="h-12 px-3 bg-header border-b border-border flex items-center gap-2 shrink-0">
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

      {/* Request Config Tabs & Body Type */}
      <PaneHeader
        tabs={[
          { value: 'params', label: 'Params', count: countActive(request.params) },
          { value: 'headers', label: 'Headers', count: countActive(request.headers) },
          { value: 'body', label: 'Body' },
          { value: 'auto-extract', label: 'Auto-Extract', count: request.extractRules?.length },
        ]}
        activeTab={activeTab}
        onTabChange={(val) => setActiveTab(val as typeof activeTab)}
        right={
          <>
            {activeTab === 'body' && (
              <Select
                value={request.bodyType}
                onChange={(e) => handleBodyTypeChange(e.target.value)}
                options={BODY_TYPE_OPTIONS}
                sizeVariant="xs"
                className="py-0.5"
                title="Body type (Multipart = multipart/form-data, URL-Encoded = x-www-form-urlencoded)"
              />
            )}
            {activeTab === 'body' && request.bodyType === 'json' && (
              <button
                onClick={() => updateTab({ ...request, bodyContent: tryPrettifyJson(request.bodyContent || '') })}
                className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs font-medium"
                title="Format JSON with 2-space indentation"
              >
                <MingCuteIcon name="code_line" size={13} />
                <span>Format</span>
              </button>
            )}
          </>
        }
      />

      {/* Tab Panels */}
      <div className={`flex-1 ${activeTab === 'body' ? 'overflow-hidden' : 'p-2 overflow-auto'}`}>
        {activeTab === 'params' && (
          <KeyValueEditor
            items={request.params || []}
            onChange={handleParamsChange}
            keyPlaceholder="Parameter Key"
            valuePlaceholder="Parameter Value"
          />
        )}

        {activeTab === 'headers' && (
          <div className="space-y-2">
            <div className="flex items-center pb-1 text-xs">
              <div className="flex items-center gap-1.5 text-muted-foreground text-2xs">
                <MingCuteIcon name="alert_line" size={13} className="text-primary shrink-0" />
                <span>
                  <strong>Tip:</strong> If <code>Content-Type</code> is disabled, it will be <strong>auto-detected</strong> from the body format.
                </span>
              </div>
            </div>
            <KeyValueEditor
              items={request.headers || []}
              onChange={(headers) => updateTab({ ...request, headers })}
              keyPlaceholder="Header Name (e.g. Authorization)"
              valuePlaceholder="Header Value (e.g. Bearer token)"
              standardHeaders={standardHeaders}
            />
          </div>
        )}

        {activeTab === 'body' && (
          <div className="h-full flex flex-col overflow-hidden">
            {request.bodyType === 'urlencoded' || request.bodyType === 'x-www-form-urlencoded' ? (
              <div className="flex-1 overflow-y-auto p-2">
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
              <div className="flex-1 overflow-y-auto p-2">
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
              <div className="flex-1 overflow-hidden">
                <CodeEditor
                  value={request.bodyContent || ''}
                  onChange={(bodyContent) => updateTab({ ...request, bodyContent })}
                  language={request.bodyType === 'json' ? 'json' : 'plaintext'}
                  bare
                />
              </div>
            ) : (
              <div className="p-4 text-muted-foreground italic">This request has no body.</div>
            )}
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
