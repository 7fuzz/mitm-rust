import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { Select, Button } from '../../common/ui';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { MultipartEditor } from '../../common/MultipartEditor';
import { UrlEncodedEditor } from '../../common/UrlEncodedEditor';
import { ExtractRulesEditor } from '../../common/ExtractRulesEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { RequestItem, HeaderItem, ParamItem, ExtractRuleItem } from '../../../services/tauri/bridge';
import type { MultipartField, UrlEncodedParam } from '../../../types';

const HTTP_METHODS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
] as const;

interface CollectionRequestTabProps {
  request: RequestItem;
}

export const CollectionRequestTab: React.FC<CollectionRequestTabProps> = ({ request }) => {
  const { updateRequestDetails, executeRequest, isExecuting } = useCollectionStore();

  const [method, setMethod] = useState(request.method);
  const [url, setUrl] = useState(request.url);
  const [activeTab, setActiveTab] = useState<'params' | 'headers' | 'body' | 'extract_rules'>('params');
  const [headers, setHeaders] = useState<HeaderItem[]>(request.headers || []);
  const [params, setParams] = useState<ParamItem[]>(request.params || []);
  const [extractRules, setExtractRules] = useState<ExtractRuleItem[]>(request.extractRules || []);
  const [bodyType, setBodyType] = useState<string>(request.bodyType || 'none');
  const [bodyContent, setBodyContent] = useState<string>(request.bodyContent || '');

  useEffect(() => {
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(request.headers || []);
    setParams(request.params || []);
    setExtractRules(request.extractRules || []);
    setBodyType(request.bodyType || 'none');
    setBodyContent(request.bodyContent || '');
  }, [
    request.id,
    request.updatedAtMs,
    request.extractRules,
    request.headers,
    request.params,
    request.method,
    request.url,
    request.bodyType,
    request.bodyContent,
  ]);

  const parseMultipartFields = (jsonStr: string): MultipartField[] => {
    try {
      const parsed = JSON.parse(jsonStr);
      if (parsed && Array.isArray(parsed.__form_data)) return parsed.__form_data;
      if (Array.isArray(parsed)) return parsed;
    } catch {}
    return [];
  };

  const parseUrlEncodedParams = (jsonStr: string): UrlEncodedParam[] => {
    try {
      const parsed = JSON.parse(jsonStr);
      if (Array.isArray(parsed)) return parsed;
    } catch {}
    return [];
  };

  const handleMultipartChange = (fields: MultipartField[]) => {
    const jsonStr = JSON.stringify({ __form_data: fields }, null, 2);
    setBodyContent(jsonStr);
    handleUpdateStore({ bodyContent: jsonStr });
  };

  const handleUrlEncodedChange = (newParams: UrlEncodedParam[]) => {
    const jsonStr = JSON.stringify(newParams, null, 2);
    setBodyContent(jsonStr);
    handleUpdateStore({ bodyContent: jsonStr });
  };

  const handleSaveAndSend = async () => {
    const updated: RequestItem = {
      ...request,
      method,
      url,
      headers,
      params,
      extractRules,
      bodyType,
      bodyContent,
      updatedAtMs: Date.now(),
    };
    await updateRequestDetails(updated);
    await executeRequest(request.id);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        const activeEl = document.activeElement;
        const isInputOrTextArea =
          activeEl &&
          (activeEl.tagName === 'INPUT' ||
            activeEl.tagName === 'TEXTAREA' ||
            (activeEl as HTMLElement).isContentEditable ||
            activeEl.getAttribute('role') === 'textbox');

        // Ctrl+Enter or Cmd+Enter always sends request
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          handleSaveAndSend();
          return;
        }

        // Plain Enter sends request if NOT focused inside an input/textarea
        if (!isInputOrTextArea) {
          e.preventDefault();
          handleSaveAndSend();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [method, url, headers, params, extractRules, bodyType, bodyContent, request]);

  const handleUpdateStore = (overrides?: Partial<RequestItem>) => {
    updateRequestDetails({
      ...request,
      method: overrides?.method ?? method,
      url: overrides?.url ?? url,
      headers: overrides?.headers ?? headers,
      params: overrides?.params ?? params,
      extractRules: overrides?.extractRules ?? extractRules,
      bodyType: overrides?.bodyType ?? bodyType,
      bodyContent: overrides?.bodyContent ?? bodyContent,
      updatedAtMs: Date.now(),
    });
  };

  const isLoading = isExecuting[request.id] || false;

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Top Address Bar: Method Select + URL Bar + Send Button */}
      <div className="p-2.5 bg-header border-b border-border flex items-center gap-2 shrink-0 font-mono">
        <div className="w-28 shrink-0 font-bold">
          <Select
            value={method}
            onChange={(e) => {
              const next = e.target.value;
              setMethod(next);
              handleUpdateStore({ method: next });
            }}
            options={HTTP_METHODS}
          />
        </div>

        <div className="flex-1 relative flex items-center">
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                handleSaveAndSend();
              }
            }}
            onBlur={() => handleUpdateStore({ url })}
            placeholder="https://api.example.com/v1/resource or {{BASE_URL}}/endpoint"
            className="w-full bg-background border border-border rounded-md px-3 py-1.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary shadow-2xs"
          />
        </div>

        <Button
          variant="primary"
          icon="send_plane_line"
          onClick={handleSaveAndSend}
          disabled={isLoading}
          className="shadow-2xs font-semibold px-4 py-1.5 shrink-0"
        >
          {isLoading ? 'Sending...' : 'Send'}
        </Button>
      </div>

      {/* Request Config Tabs Bar */}
      <div className="bg-header border-b border-border px-3 py-1 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-1">
          {(['params', 'headers', 'body', 'extract_rules'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-3 py-1 rounded text-xs font-medium uppercase transition-colors cursor-pointer ${
                activeTab === tab
                  ? 'bg-surface text-primary border border-border shadow-2xs font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab === 'headers'
                ? `Headers (${headers.length})`
                : tab === 'extract_rules'
                ? `Extract Rules (${extractRules.length})`
                : tab}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Panels */}
      <div className="flex-1 p-3 overflow-auto">
        {activeTab === 'params' && (
          <KeyValueEditor
            items={params}
            onChange={(newParams) => {
              setParams(newParams);
              handleUpdateStore({ params: newParams });
            }}
            keyPlaceholder="URL Param Key"
            valuePlaceholder="Value"
          />
        )}

        {activeTab === 'headers' && (
          <KeyValueEditor
            items={headers}
            onChange={(newHeaders) => {
              setHeaders(newHeaders);
              handleUpdateStore({ headers: newHeaders });
            }}
            keyPlaceholder="Header Name (e.g. Authorization)"
            valuePlaceholder="Header Value (e.g. Bearer {{TOKEN}})"
          />
        )}

        {activeTab === 'extract_rules' && (
          <ExtractRulesEditor
            rules={extractRules}
            onChange={(newRules) => {
              setExtractRules(newRules);
              handleUpdateStore({ extractRules: newRules });
            }}
          />
        )}

        {activeTab === 'body' && (
          <div className="h-full flex flex-col gap-2">
            <div className="flex items-center gap-3 font-mono text-xs text-muted-foreground pb-1">
              {[
                { id: 'none', label: 'none' },
                { id: 'json', label: 'json' },
                { id: 'raw', label: 'raw' },
                { id: 'form-data', label: 'form-data' },
                { id: 'urlencoded', label: 'x-www-form-urlencoded' },
              ].map((opt) => (
                <label key={opt.id} className="flex items-center gap-1.5 cursor-pointer">
                  <input
                    type="radio"
                    name={`body-type-${request.id}`}
                    value={opt.id}
                    checked={bodyType === opt.id}
                    onChange={(e) => {
                      const nextType = e.target.value;
                      setBodyType(nextType);
                      handleUpdateStore({ bodyType: nextType });
                    }}
                    className="accent-primary cursor-pointer"
                  />
                  <span className={bodyType === opt.id ? 'text-foreground font-semibold' : ''}>{opt.label}</span>
                </label>
              ))}
            </div>

            {bodyType === 'urlencoded' || bodyType === 'x-www-form-urlencoded' ? (
              <div className="flex-1 overflow-y-auto">
                <UrlEncodedEditor
                  params={parseUrlEncodedParams(bodyContent)}
                  onChange={handleUrlEncodedChange}
                />
              </div>
            ) : bodyType === 'form-data' || bodyType === 'multipart' ? (
              <div className="flex-1 overflow-y-auto">
                <MultipartEditor
                  fields={parseMultipartFields(bodyContent)}
                  onChange={handleMultipartChange}
                />
              </div>
            ) : bodyType !== 'none' ? (
              <div className="flex-1 border border-border rounded-lg overflow-hidden bg-background">
                <CodeEditor
                  value={bodyContent}
                  onChange={(val) => {
                    setBodyContent(val);
                    handleUpdateStore({ bodyContent: val });
                  }}
                  language={bodyType === 'json' ? 'json' : 'plaintext'}
                />
              </div>
            ) : (
              <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic text-xs">
                <MingCuteIcon name="file_text_line" size={32} className="opacity-30 mb-1" />
                This request does not have a body.
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
