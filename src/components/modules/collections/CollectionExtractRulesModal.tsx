import React, { useState, useEffect } from 'react';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button } from '../../common/ui';
import { useCollectionStore } from '../../../stores/useCollectionStore';
import type { CollectionTreeItem, RequestItem, ExtractRuleItem } from '../../../services/tauri/bridge';

interface CollectionExtractRulesModalProps {
  isOpen: boolean;
  onClose: () => void;
  collection?: CollectionTreeItem | null;
  request?: RequestItem | null;
}

export const CollectionExtractRulesModal: React.FC<CollectionExtractRulesModalProps> = ({
  isOpen,
  onClose,
  collection,
  request: initialRequest,
}) => {
  const { collectionsTree, updateRequestDetails } = useCollectionStore();

  // All requests within the target collection
  const allCollectionRequests = React.useMemo(() => {
    if (initialRequest) return [initialRequest];
    if (!collection) return [];
    const extractReqs = (col: CollectionTreeItem): RequestItem[] => {
      const direct = col.requests || [];
      const nested = (col.children || []).flatMap(extractReqs);
      return [...direct, ...nested];
    };
    return extractReqs(collection);
  }, [collection, initialRequest, collectionsTree]);

  const [selectedReqId, setSelectedReqId] = useState<string>('');
  const [rules, setRules] = useState<ExtractRuleItem[]>([]);

  useEffect(() => {
    if (isOpen) {
      if (initialRequest) {
        setSelectedReqId(initialRequest.id);
        setRules(initialRequest.extractRules || []);
      } else if (allCollectionRequests.length > 0) {
        setSelectedReqId(allCollectionRequests[0].id);
        setRules(allCollectionRequests[0].extractRules || []);
      } else {
        setSelectedReqId('');
        setRules([]);
      }
    }
  }, [isOpen, initialRequest, collection, allCollectionRequests]);

  const activeRequest = allCollectionRequests.find((r) => r.id === selectedReqId);

  const handleSelectRequest = (reqId: string) => {
    setSelectedReqId(reqId);
    const target = allCollectionRequests.find((r) => r.id === reqId);
    setRules(target?.extractRules || []);
  };

  const handleAddRule = () => {
    const newRule: ExtractRuleItem = {
      id: crypto.randomUUID(),
      type: 'json',
      targetVariable: 'new_variable',
      expression: 'data.id',
      enabled: true,
    };
    setRules([...rules, newRule]);
  };

  const handleUpdateRule = (index: number, updates: Partial<ExtractRuleItem>) => {
    const updated = rules.map((r, i) => (i === index ? { ...r, ...updates } : r));
    setRules(updated);
  };

  const handleDeleteRule = (index: number) => {
    setRules(rules.filter((_, i) => i !== index));
  };

  const handleSave = async () => {
    if (!activeRequest) return;
    const updatedReq: RequestItem = {
      ...activeRequest,
      extractRules: rules,
      updatedAtMs: Date.now(),
    };
    await updateRequestDetails(updatedReq);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs animate-in fade-in duration-150 font-sans text-xs select-none cursor-pointer"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-2xl bg-surface border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] text-foreground cursor-default select-text"
      >
        {/* Header */}
        <div className="p-3 bg-header border-b border-border flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="lightning_line" size={18} className="text-amber-500" />
            <div>
              <span className="font-semibold text-sm">Auto-Extract Rules</span>
              <span className="text-muted-foreground ml-2 text-xs">
                {collection ? `Collection: ${collection.name}` : activeRequest?.name}
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors cursor-pointer"
          >
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {/* Content Body */}
        <div className="flex-1 overflow-hidden flex flex-col md:flex-row">
          {/* Request Selector Sidebar if Collection has multiple requests */}
          {allCollectionRequests.length > 1 && (
            <div className="w-full md:w-56 bg-background border-r border-border p-2 overflow-y-auto space-y-1 shrink-0">
              <label className="text-3xs font-semibold uppercase text-muted-foreground tracking-wider px-1 block mb-1 font-mono">
                Requests ({allCollectionRequests.length})
              </label>
              {allCollectionRequests.map((req) => {
                const isSelected = req.id === selectedReqId;
                const ruleCount = (req.extractRules || []).length;
                return (
                  <button
                    key={req.id}
                    onClick={() => handleSelectRequest(req.id)}
                    className={`w-full flex items-center justify-between p-2 rounded-lg text-left transition-colors cursor-pointer ${
                      isSelected
                        ? 'bg-primary/10 border border-primary/40 text-foreground font-semibold'
                        : 'hover:bg-neutral-subtle/70 text-muted-foreground'
                    }`}
                  >
                    <div className="truncate min-w-0 flex-1">
                      <div className="truncate text-xs">{req.name}</div>
                      <div className="text-3xs text-muted-foreground/80 font-mono truncate">{req.method} {req.url}</div>
                    </div>
                    {ruleCount > 0 && (
                      <span className="ml-1.5 px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-400 font-mono text-3xs font-bold shrink-0">
                        {ruleCount}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          )}

          {/* Rules Editor Panel */}
          <div className="flex-1 p-3 flex flex-col overflow-y-auto space-y-3">
            {activeRequest ? (
              <>
                <div className="flex items-center justify-between pb-1 border-b border-border">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-foreground text-xs">{activeRequest.name}</span>
                    <span className="text-2xs font-mono text-muted-foreground">({rules.length} rules)</span>
                  </div>
                  <Button
                    variant="outline"
                    sizeVariant="xs"
                    icon="add_line"
                    onClick={handleAddRule}
                  >
                    Add Extract Rule
                  </Button>
                </div>

                {rules.length === 0 ? (
                  <div className="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2">
                    <MingCuteIcon name="lightning_line" size={32} className="opacity-30" />
                    <span>No auto-extraction rules configured for this request</span>
                    <span className="text-2xs">
                      Extracted values automatically populate the active environment's <code className="text-amber-500">(auto)</code> variable.
                    </span>
                    <Button variant="primary" sizeVariant="xs" icon="plus_line" onClick={handleAddRule}>
                      Create First Rule
                    </Button>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {rules.map((rule, idx) => (
                      <div
                        key={rule.id || idx}
                        className="p-2.5 rounded-lg border border-border bg-background flex flex-col gap-2 font-mono text-xs"
                      >
                        <div className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={rule.enabled !== false}
                            onChange={(e) => handleUpdateRule(idx, { enabled: e.target.checked })}
                            className="rounded border-border text-primary cursor-pointer"
                            title="Toggle Rule"
                          />
                          <select
                            value={rule.type || 'json'}
                            onChange={(e) => handleUpdateRule(idx, { type: e.target.value })}
                            className="bg-surface border border-border rounded px-2 py-1 text-xs text-foreground focus:outline-none"
                          >
                            <option value="json">JSONPath / Dot Path</option>
                            <option value="header">Response Header</option>
                            <option value="after_string">After String</option>
                            <option value="before_string">Before String</option>
                            <option value="between_string">Between String</option>
                            <option value="body_regex">Regex Pattern</option>
                          </select>

                          <div className="flex-1 flex items-center gap-1.5 bg-surface border border-border rounded px-2 py-1">
                            <span className="text-3xs text-muted-foreground font-sans font-bold">Target:</span>
                            <input
                              type="text"
                              value={rule.targetVariable}
                              onChange={(e) => handleUpdateRule(idx, { targetVariable: e.target.value })}
                              placeholder="variable_name"
                              className="w-full bg-transparent text-xs text-amber-500 font-bold focus:outline-none"
                            />
                          </div>

                          <button
                            onClick={() => handleDeleteRule(idx)}
                            className="p-1 text-muted-foreground hover:text-rose-500 rounded transition-colors cursor-pointer"
                            title="Delete Rule"
                          >
                            <MingCuteIcon name="close_line" size={14} />
                          </button>
                        </div>

                        <div className="flex items-center gap-1.5 bg-surface border border-border rounded px-2 py-1">
                          <span className="text-3xs text-muted-foreground font-sans font-bold">Expr:</span>
                          <input
                            type="text"
                            value={rule.expression}
                            onChange={(e) => handleUpdateRule(idx, { expression: e.target.value })}
                            placeholder={
                              rule.type === 'header'
                                ? 'Header name (e.g. Set-Cookie)'
                                : rule.type === 'after_string'
                                ? 'prefix||max_length'
                                : rule.type === 'between_string'
                                ? 'start_delim||end_delim'
                                : rule.type === 'body_regex'
                                ? 'token=([a-zA-Z0-9_-]+)'
                                : 'data.0.id or access_token'
                            }
                            className="w-full bg-transparent text-xs text-foreground focus:outline-none"
                          />
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </>
            ) : (
              <div className="py-12 text-center text-muted-foreground italic">
                No requests available in this collection
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="p-3 bg-header border-t border-border flex items-center justify-between">
          <div className="text-2xs text-muted-foreground flex items-center gap-1">
            <MingCuteIcon name="information_line" size={14} className="text-primary" />
            <span>Extracted values automatically update the <strong>(auto)</strong> variant on send.</span>
          </div>

          <div className="flex items-center gap-2">
            <Button variant="outline" sizeVariant="xs" onClick={onClose}>
              Cancel
            </Button>
            <Button variant="primary" sizeVariant="xs" icon="check_line" onClick={handleSave}>
              Save Rules
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
