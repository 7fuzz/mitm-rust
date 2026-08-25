import React, { useState } from 'react';
import type { RepeaterTab } from '../../../services/tauri/bridge';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { CodeEditor } from '../../common/CodeEditor';
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

  const executing = isExecuting[request.id] || false;

  const handleMethodChange = (method: string) => {
    updateTab({ ...request, method });
  };

  const handleUrlChange = (url: string) => {
    updateTab({ ...request, url });
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

      {/* Request Config Tabs */}
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
          <KeyValueEditor
            items={request.headers || []}
            onChange={(headers) => updateTab({ ...request, headers })}
            keyPlaceholder="Header Name"
            valuePlaceholder="Header Value"
          />
        )}

        {activeTab === 'body' && (
          <div className="h-full flex flex-col gap-2">
            <div className="flex items-center gap-2 text-xs">
              <span className="text-muted-foreground">Body Type:</span>
              {(['none', 'json', 'raw'] as const).map((bType) => (
                <label key={bType} className="flex items-center gap-1 cursor-pointer">
                  <input
                    type="radio"
                    name={`bodyType-${request.id}`}
                    checked={request.bodyType === bType}
                    onChange={() => updateTab({ ...request, bodyType: bType })}
                    className="text-primary"
                  />
                  <span className="capitalize font-mono">{bType}</span>
                </label>
              ))}
            </div>

            {request.bodyType !== 'none' && (
              <div className="flex-1 overflow-hidden">
                <CodeEditor
                  value={request.bodyContent || ''}
                  onChange={(bodyContent) => updateTab({ ...request, bodyContent })}
                  language={request.bodyType === 'json' ? 'json' : 'plaintext'}
                  readOnly={false}
                />
              </div>
            )}
          </div>
        )}

        {activeTab === 'auto-extract' && (
          <div className="space-y-3">
            <span className="font-semibold text-foreground block">Variable Auto-Extraction Rules</span>
            <p className="text-muted-foreground text-xs">
              Configure JSONPath or Regex extractions to automatically populate variables upon response.
            </p>
            <div className="border border-border rounded overflow-hidden">
              <table className="w-full text-left font-mono text-xs">
                <thead className="bg-header border-b border-border text-muted-foreground">
                  <tr>
                    <th className="px-3 py-1.5">Type</th>
                    <th className="px-3 py-1.5">Expression</th>
                    <th className="px-3 py-1.5">Target Variable</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border bg-surface">
                  {(request.extractRules || []).map((rule, idx) => (
                    <tr key={rule.id || idx}>
                      <td className="px-3 py-1.5 uppercase font-bold text-primary">{rule.type}</td>
                      <td className="px-3 py-1.5">{rule.expression}</td>
                      <td className="px-3 py-1.5 text-emerald-500 font-bold">{`{{${rule.targetVariable}}}`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
