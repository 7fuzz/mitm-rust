import React, { useState, useEffect } from 'react';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import { useSettingsStore } from '../../../stores/useSettingsStore';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
import { ExtractRulesEditor } from '../../common/ExtractRulesEditor';
import { MarkdownViewer } from '../../common/MarkdownViewer';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { PaneHeader } from '../../common/PaneHeader';
import type { RequestItem, HeaderItem, ParamItem, ExtractRuleItem, RequestPreview } from '../../../services/tauri/bridge';
import { previewCollectionRequest } from '../../../services/tauri/bridge';
import { CollectionAddressBar } from './tab/CollectionAddressBar';
import { CollectionBodyEditor } from './tab/CollectionBodyEditor';
import { CollectionInterpolationTab } from './tab/CollectionInterpolationTab';

const normalizeBodyType = (type?: string): string => {
  if (!type) return 'none';
  if (type === 'multipart') return 'form-data';
  if (type === 'x-www-form-urlencoded') return 'urlencoded';
  return type;
};

interface CollectionRequestTabProps {
  request: RequestItem;
}

export const CollectionRequestTab: React.FC<CollectionRequestTabProps> = ({ request }) => {
  const { updateRequestDetails, executeRequest, isExecuting, isHistoryDrawerOpen, toggleHistoryDrawer } = useCollectionStore();

  const [activeTab, setActiveTab] = useState<'docs' | 'params' | 'headers' | 'body' | 'extract_rules' | 'interpolation'>('params');
  const [docsMode, setDocsMode] = useState<'preview' | 'edit' | 'split'>('preview');

  // Local state for instant editing
  const [method, setMethod] = useState(request.method || 'GET');
  const [url, setUrl] = useState(request.url || '');
  const [headers, setHeaders] = useState<HeaderItem[]>(request.headers || []);
  const [params, setParams] = useState<ParamItem[]>(request.params || []);
  const [extractRules, setExtractRules] = useState<ExtractRuleItem[]>(request.extractRules || []);
  const [bodyType, setBodyType] = useState<string>(normalizeBodyType(request.bodyType));
  const [bodyJson, setBodyJson] = useState(request.bodyJson || '{\n  \n}');
  const [bodyRaw, setBodyRaw] = useState(request.bodyRaw || '');
  const [bodyFormData, setBodyFormData] = useState(request.bodyFormData || '[]');
  const [bodyUrlencoded, setBodyUrlencoded] = useState(request.bodyUrlencoded || '[]');
  const [description, setDescription] = useState(request.description || '');

  // Interpolation preview state
  const [preview, setPreview] = useState<RequestPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);

  // Sync if external request object changes
  useEffect(() => {
    setMethod(request.method || 'GET');
    setUrl(request.url || '');
    setHeaders(request.headers || []);
    setParams(request.params || []);
    setExtractRules(request.extractRules || []);
    setBodyType(normalizeBodyType(request.bodyType));
    setBodyJson(request.bodyJson || '{\n  \n}');
    setBodyRaw(request.bodyRaw || '');
    setBodyFormData(request.bodyFormData || '[]');
    setBodyUrlencoded(request.bodyUrlencoded || '[]');
    setDescription(request.description || '');
  }, [request.id]);

  const handleUpdateStore = (overrides?: Partial<RequestItem>) => {
    updateRequestDetails({
      ...request,
      method: overrides?.method ?? method,
      url: overrides?.url ?? url,
      headers: overrides?.headers ?? headers,
      params: overrides?.params ?? params,
      extractRules: overrides?.extractRules ?? extractRules,
      bodyType: overrides?.bodyType ?? bodyType,
      bodyJson: overrides?.bodyJson ?? bodyJson,
      bodyRaw: overrides?.bodyRaw ?? bodyRaw,
      bodyFormData: overrides?.bodyFormData ?? bodyFormData,
      bodyUrlencoded: overrides?.bodyUrlencoded ?? bodyUrlencoded,
      description: overrides?.description ?? description,
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
      extractRules,
      bodyType,
      bodyJson,
      bodyRaw,
      bodyFormData,
      bodyUrlencoded,
      description,
      updatedAtMs: Date.now(),
    };
    await updateRequestDetails(updated);
    await executeRequest(request.id);
  };

  const handleFetchPreview = () => {
    handleUpdateStore();
    setPreviewLoading(true);
    setPreviewError(null);
    previewCollectionRequest(request.id)
      .then(setPreview)
      .catch((e) => setPreviewError(String(e)))
      .finally(() => setPreviewLoading(false));
  };

  // Keyboard shortcut: only Cmd/Ctrl+Enter triggers request execution, ignoring if dialogs are open
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        const isModalOpen =
          useSettingsStore.getState().isQuickVarModalOpen ||
          document.querySelector('.fixed.z-50') !== null ||
          document.activeElement?.closest('.fixed, [role="dialog"]') !== null;

        if (!isModalOpen) {
          e.preventDefault();
          handleSaveAndSend();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [method, url, headers, params, extractRules, bodyType, bodyJson, bodyRaw, bodyFormData, bodyUrlencoded, description, request]);

  const isLoading = isExecuting[request.id] || false;

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Top Address Bar */}
      <CollectionAddressBar
        method={method}
        url={url}
        isLoading={isLoading}
        onMethodChange={(next) => {
          setMethod(next);
          handleUpdateStore({ method: next });
        }}
        onUrlChange={setUrl}
        onUrlBlur={() => handleUpdateStore({ url })}
        onSend={handleSaveAndSend}
        isHistoryOpen={isHistoryDrawerOpen}
        onToggleHistory={() => toggleHistoryDrawer()}
      />

      {/* Request Config Tabs */}
      <PaneHeader
        tabs={[
          { value: 'docs', label: description ? 'Docs •' : 'Docs' },
          { value: 'params', label: 'Params', count: params.length },
          { value: 'headers', label: 'Headers', count: headers.length },
          { value: 'body', label: 'Body' },
          { value: 'extract_rules', label: 'Extract Rules', count: extractRules.length },
          { value: 'interpolation', label: 'Interpolation' },
        ]}
        activeTab={activeTab}
        onTabChange={(val) => {
          setActiveTab(val as typeof activeTab);
          if (val === 'interpolation') {
            handleFetchPreview();
          }
        }}
      />

      {/* Tab Panels */}
      <div className="flex-1 p-3 overflow-auto">
        {activeTab === 'docs' && (
          <div className="h-full flex flex-col gap-2">
            <div className="flex items-center justify-between font-mono text-xs text-muted-foreground pb-1 shrink-0">
              <div className="flex items-center gap-1 bg-background p-0.5 rounded border border-border">
                {(['preview', 'edit', 'split'] as const).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    onClick={() => setDocsMode(mode)}
                    className={`px-2.5 py-0.5 rounded text-2xs font-semibold uppercase transition-colors cursor-pointer ${
                      docsMode === mode
                        ? 'bg-primary text-primary-foreground shadow-2xs'
                        : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
                    }`}
                  >
                    {mode}
                  </button>
                ))}
              </div>

              {description && (
                <button
                  type="button"
                  onClick={() => navigator.clipboard.writeText(description)}
                  className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-foreground text-2xs cursor-pointer transition-colors"
                  title="Copy Markdown"
                >
                  <MingCuteIcon name="copy_line" size={12} />
                  <span>Copy MD</span>
                </button>
              )}
            </div>

            <div className="flex-1 min-h-0 border border-border rounded-lg overflow-hidden bg-background">
              {docsMode === 'edit' ? (
                <CodeEditor
                  value={description}
                  onChange={(val) => {
                    setDescription(val);
                    handleUpdateStore({ description: val });
                  }}
                  language="markdown"
                />
              ) : docsMode === 'split' ? (
                <div className="h-full flex divide-x divide-border overflow-hidden">
                  <div className="w-1/2 h-full overflow-hidden">
                    <CodeEditor
                      value={description}
                      onChange={(val) => {
                        setDescription(val);
                        handleUpdateStore({ description: val });
                      }}
                      language="markdown"
                    />
                  </div>
                  <div className="w-1/2 h-full overflow-y-auto bg-surface">
                    <MarkdownViewer
                      content={description}
                      onEdit={() => setDocsMode('edit')}
                    />
                  </div>
                </div>
              ) : (
                <div className="h-full overflow-y-auto bg-surface">
                  <MarkdownViewer
                    content={description}
                    onEdit={() => setDocsMode('edit')}
                  />
                </div>
              )}
            </div>
          </div>
        )}

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
          <CollectionBodyEditor
            requestId={request.id}
            bodyType={bodyType}
            bodyJson={bodyJson}
            bodyRaw={bodyRaw}
            bodyFormData={bodyFormData}
            bodyUrlencoded={bodyUrlencoded}
            onBodyTypeChange={(next) => {
              setBodyType(next);
              handleUpdateStore({ bodyType: next });
            }}
            onBodyJsonChange={(val) => {
              setBodyJson(val);
              handleUpdateStore({ bodyJson: val });
            }}
            onBodyRawChange={(val) => {
              setBodyRaw(val);
              handleUpdateStore({ bodyRaw: val });
            }}
            onBodyFormDataChange={(val) => {
              setBodyFormData(val);
              handleUpdateStore({ bodyFormData: val });
            }}
            onBodyUrlencodedChange={(val) => {
              setBodyUrlencoded(val);
              handleUpdateStore({ bodyUrlencoded: val });
            }}
          />
        )}

        {activeTab === 'interpolation' && (
          <CollectionInterpolationTab
            preview={preview}
            isLoading={previewLoading}
            error={previewError}
            onRefresh={handleFetchPreview}
          />
        )}
      </div>
    </div>
  );
};
