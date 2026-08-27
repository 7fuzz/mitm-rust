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
import {
  convertJsonToFormData,
  convertJsonToUrlEncoded,
  convertFormDataToJson,
  convertUrlEncodedToJson,
  convertRawToJson,
  convertFormDataToUrlEncoded,
  convertUrlEncodedToFormData,
} from '../../../utils/bodyConverters';

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

  // 4 Separate Body Buffers
  const [bodyJson, setBodyJson] = useState<string>(
    request.bodyJson ?? (request.bodyType === 'json' ? request.bodyContent || '' : '')
  );
  const [bodyRaw, setBodyRaw] = useState<string>(
    request.bodyRaw ?? (request.bodyType === 'raw' ? request.bodyContent || '' : request.bodyContent || '')
  );
  const [bodyFormData, setBodyFormData] = useState<string>(
    request.bodyFormData ?? (request.bodyType === 'form-data' || request.bodyType === 'multipart' ? request.bodyContent || '' : '')
  );
  const [bodyUrlencoded, setBodyUrlencoded] = useState<string>(
    request.bodyUrlencoded ?? (request.bodyType === 'urlencoded' || request.bodyType === 'x-www-form-urlencoded' ? request.bodyContent || '' : '')
  );

  const [isConvertMenuOpen, setIsConvertMenuOpen] = useState(false);

  useEffect(() => {
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(request.headers || []);
    setParams(request.params || []);
    setExtractRules(request.extractRules || []);
    setBodyType(request.bodyType || 'none');
    setBodyContent(request.bodyContent || '');
    setBodyJson(request.bodyJson ?? (request.bodyType === 'json' ? request.bodyContent || '' : ''));
    setBodyRaw(request.bodyRaw ?? (request.bodyType === 'raw' ? request.bodyContent || '' : request.bodyContent || ''));
    setBodyFormData(request.bodyFormData ?? (request.bodyType === 'form-data' || request.bodyType === 'multipart' ? request.bodyContent || '' : ''));
    setBodyUrlencoded(request.bodyUrlencoded ?? (request.bodyType === 'urlencoded' || request.bodyType === 'x-www-form-urlencoded' ? request.bodyContent || '' : ''));
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
    request.bodyJson,
    request.bodyRaw,
    request.bodyFormData,
    request.bodyUrlencoded,
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
    setBodyFormData(jsonStr);
    setBodyContent(jsonStr);
    handleUpdateStore({ bodyFormData: jsonStr, bodyContent: jsonStr });
  };

  const handleUrlEncodedChange = (newParams: UrlEncodedParam[]) => {
    const jsonStr = JSON.stringify(newParams, null, 2);
    setBodyUrlencoded(jsonStr);
    setBodyContent(jsonStr);
    handleUpdateStore({ bodyUrlencoded: jsonStr, bodyContent: jsonStr });
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
      bodyJson,
      bodyRaw,
      bodyFormData,
      bodyUrlencoded,
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
    const nextBodyType = overrides?.bodyType ?? bodyType;
    const nextBodyJson = overrides?.bodyJson ?? bodyJson;
    const nextBodyRaw = overrides?.bodyRaw ?? bodyRaw;
    const nextBodyFormData = overrides?.bodyFormData ?? bodyFormData;
    const nextBodyUrlencoded = overrides?.bodyUrlencoded ?? bodyUrlencoded;

    // Determine active body content string based on bodyType
    let nextBodyContent = overrides?.bodyContent;
    if (nextBodyContent === undefined) {
      if (nextBodyType === 'json') nextBodyContent = nextBodyJson;
      else if (nextBodyType === 'raw') nextBodyContent = nextBodyRaw;
      else if (nextBodyType === 'form-data' || nextBodyType === 'multipart') nextBodyContent = nextBodyFormData;
      else if (nextBodyType === 'urlencoded' || nextBodyType === 'x-www-form-urlencoded') nextBodyContent = nextBodyUrlencoded;
      else nextBodyContent = '';
    }

    updateRequestDetails({
      ...request,
      method: overrides?.method ?? method,
      url: overrides?.url ?? url,
      headers: overrides?.headers ?? headers,
      params: overrides?.params ?? params,
      extractRules: overrides?.extractRules ?? extractRules,
      bodyType: nextBodyType,
      bodyContent: nextBodyContent,
      bodyJson: nextBodyJson,
      bodyRaw: nextBodyRaw,
      bodyFormData: nextBodyFormData,
      bodyUrlencoded: nextBodyUrlencoded,
      updatedAtMs: Date.now(),
    });
  };

  // Format Conversion Handler
  const handleConvert = async (type: string) => {
    setIsConvertMenuOpen(false);

    if (type === 'json_to_formdata') {
      const converted = convertJsonToFormData(bodyJson || bodyContent);
      setBodyFormData(converted);
      setBodyType('form-data');
      setBodyContent(converted);
      handleUpdateStore({ bodyFormData: converted, bodyType: 'form-data', bodyContent: converted });
    } else if (type === 'json_to_urlencoded') {
      const converted = convertJsonToUrlEncoded(bodyJson || bodyContent);
      setBodyUrlencoded(converted);
      setBodyType('urlencoded');
      setBodyContent(converted);
      handleUpdateStore({ bodyUrlencoded: converted, bodyType: 'urlencoded', bodyContent: converted });
    } else if (type === 'formdata_to_json') {
      const converted = await convertFormDataToJson(bodyFormData || bodyContent);
      setBodyJson(converted);
      setBodyType('json');
      setBodyContent(converted);
      handleUpdateStore({ bodyJson: converted, bodyType: 'json', bodyContent: converted });
    } else if (type === 'urlencoded_to_json') {
      const converted = convertUrlEncodedToJson(bodyUrlencoded || bodyContent);
      setBodyJson(converted);
      setBodyType('json');
      setBodyContent(converted);
      handleUpdateStore({ bodyJson: converted, bodyType: 'json', bodyContent: converted });
    } else if (type === 'raw_to_json') {
      const converted = convertRawToJson(bodyRaw || bodyContent);
      setBodyJson(converted);
      setBodyType('json');
      setBodyContent(converted);
      handleUpdateStore({ bodyJson: converted, bodyType: 'json', bodyContent: converted });
    } else if (type === 'formdata_to_urlencoded') {
      const converted = convertFormDataToUrlEncoded(bodyFormData || bodyContent);
      setBodyUrlencoded(converted);
      setBodyType('urlencoded');
      setBodyContent(converted);
      handleUpdateStore({ bodyUrlencoded: converted, bodyType: 'urlencoded', bodyContent: converted });
    } else if (type === 'urlencoded_to_formdata') {
      const converted = convertUrlEncodedToFormData(bodyUrlencoded || bodyContent);
      setBodyFormData(converted);
      setBodyType('form-data');
      setBodyContent(converted);
      handleUpdateStore({ bodyFormData: converted, bodyType: 'form-data', bodyContent: converted });
    }
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
            <div className="flex items-center justify-between font-mono text-xs text-muted-foreground pb-1">
              <div className="flex items-center gap-3">
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

                        let activeText = '';
                        if (nextType === 'json') activeText = bodyJson;
                        else if (nextType === 'raw') activeText = bodyRaw;
                        else if (nextType === 'form-data' || nextType === 'multipart') activeText = bodyFormData;
                        else if (nextType === 'urlencoded' || nextType === 'x-www-form-urlencoded') activeText = bodyUrlencoded;

                        setBodyContent(activeText);
                        handleUpdateStore({ bodyType: nextType, bodyContent: activeText });
                      }}
                      className="accent-primary cursor-pointer"
                    />
                    <span className={bodyType === opt.id ? 'text-foreground font-semibold' : ''}>{opt.label}</span>
                  </label>
                ))}
              </div>

              {/* Format Converter Dropdown */}
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsConvertMenuOpen(!isConvertMenuOpen)}
                  className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-amber-500 font-bold text-[11px] cursor-pointer transition-colors"
                  title="Auto Convert Format"
                >
                  <MingCuteIcon name="transfer_line" size={13} />
                  <span>Convert Format</span>
                  <MingCuteIcon name="down_line" size={12} />
                </button>

                {isConvertMenuOpen && (
                  <div className="absolute right-0 top-full mt-1 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 text-[11px] w-48 font-sans text-foreground flex flex-col">
                    <button
                      onClick={() => handleConvert('json_to_formdata')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-amber-500" />
                      <span>JSON ➔ Form-Data</span>
                    </button>
                    <button
                      onClick={() => handleConvert('json_to_urlencoded')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-amber-500" />
                      <span>JSON ➔ UrlEncoded</span>
                    </button>
                    <button
                      onClick={() => handleConvert('formdata_to_json')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                      <span>Form-Data ➔ JSON (Base64)</span>
                    </button>
                    <button
                      onClick={() => handleConvert('urlencoded_to_json')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                      <span>UrlEncoded ➔ JSON</span>
                    </button>
                    <button
                      onClick={() => handleConvert('raw_to_json')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                      <span>Raw ➔ JSON</span>
                    </button>
                    <div className="my-1 border-t border-border" />
                    <button
                      onClick={() => handleConvert('formdata_to_urlencoded')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-primary" />
                      <span>Form-Data ➔ UrlEncoded</span>
                    </button>
                    <button
                      onClick={() => handleConvert('urlencoded_to_formdata')}
                      className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
                    >
                      <MingCuteIcon name="right_line" size={12} className="text-primary" />
                      <span>UrlEncoded ➔ Form-Data</span>
                    </button>
                  </div>
                )}
              </div>
            </div>

            {bodyType === 'urlencoded' || bodyType === 'x-www-form-urlencoded' ? (
              <div className="flex-1 overflow-y-auto">
                <UrlEncodedEditor
                  params={parseUrlEncodedParams(bodyUrlencoded || bodyContent)}
                  onChange={handleUrlEncodedChange}
                />
              </div>
            ) : bodyType === 'form-data' || bodyType === 'multipart' ? (
              <div className="flex-1 overflow-y-auto">
                <MultipartEditor
                  fields={parseMultipartFields(bodyFormData || bodyContent)}
                  onChange={handleMultipartChange}
                />
              </div>
            ) : bodyType !== 'none' ? (
              <div className="flex-1 border border-border rounded-lg overflow-hidden bg-background">
                <CodeEditor
                  value={bodyType === 'json' ? bodyJson : bodyRaw}
                  onChange={(val) => {
                    if (bodyType === 'json') {
                      setBodyJson(val);
                      setBodyContent(val);
                      handleUpdateStore({ bodyJson: val, bodyContent: val });
                    } else {
                      setBodyRaw(val);
                      setBodyContent(val);
                      handleUpdateStore({ bodyRaw: val, bodyContent: val });
                    }
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
