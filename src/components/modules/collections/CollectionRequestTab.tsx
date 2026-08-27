import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { Select, Button } from '../../common/ui';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { MultipartEditor } from '../../common/MultipartEditor';
import { UrlEncodedEditor } from '../../common/UrlEncodedEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { RequestItem, HeaderItem, ParamItem } from '../../../services/tauri/bridge';
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
  const [activeTab, setActiveTab] = useState<'params' | 'headers' | 'body'>('params');
  const [headers, setHeaders] = useState<HeaderItem[]>(request.headers || []);
  const [params, setParams] = useState<ParamItem[]>(request.params || []);
  const [bodyType, setBodyType] = useState<string>(request.bodyType || 'none');
  const [bodyContent, setBodyContent] = useState<string>(request.bodyContent || '');

  useEffect(() => {
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(request.headers || []);
    setParams(request.params || []);
    setBodyType(request.bodyType || 'none');
    setBodyContent(request.bodyContent || '');
  }, [request.id]);

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
    updateRequestDetails({
      ...request,
      method,
      url,
      headers,
      params,
      bodyType,
      bodyContent: jsonStr,
      updatedAtMs: Date.now(),
    });
  };

  const handleUrlEncodedChange = (newParams: UrlEncodedParam[]) => {
    const jsonStr = JSON.stringify(newParams, null, 2);
    setBodyContent(jsonStr);
    updateRequestDetails({
      ...request,
      method,
      url,
      headers,
      params,
      bodyType,
      bodyContent: jsonStr,
      updatedAtMs: Date.now(),
    });
  };

  const handleSaveAndSend = async () => {
    const updated: RequestItem = {
      ...request,
      method,
      url,
      headers,
      params,
      bodyType,
      bodyContent,
      updatedAtMs: Date.now(),
    };
    await updateRequestDetails(updated);
    await executeRequest(request.id);
  };

  const handleUpdateStore = () => {
    updateRequestDetails({
      ...request,
      method,
      url,
      headers,
      params,
      bodyType,
      bodyContent,
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
              setMethod(e.target.value);
              handleUpdateStore();
            }}
            options={HTTP_METHODS}
          />
        </div>

        <div className="flex-1 relative flex items-center">
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onBlur={handleUpdateStore}
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
          {(['params', 'headers', 'body'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-3 py-1 rounded text-xs font-medium uppercase transition-colors cursor-pointer ${
                activeTab === tab
                  ? 'bg-surface text-primary border border-border shadow-2xs font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab === 'headers' ? `Headers (${headers.length})` : tab}
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
              handleUpdateStore();
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
              handleUpdateStore();
            }}
            keyPlaceholder="Header Name (e.g. Authorization)"
            valuePlaceholder="Header Value (e.g. Bearer {{TOKEN}})"
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
                      setBodyType(e.target.value);
                      handleUpdateStore();
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
                    handleUpdateStore();
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
