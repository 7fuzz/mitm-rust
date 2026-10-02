import React, { useState } from 'react';
import type { WebhookDelivery } from '../../../../types';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { CodeEditor } from '../../../common/CodeEditor';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button, Input } from '../../../common/ui';

interface WebhookDeliveryInspectorProps {
  selectedDelivery: WebhookDelivery | null;
}

export const WebhookDeliveryInspector: React.FC<WebhookDeliveryInspectorProps> = ({
  selectedDelivery,
}) => {
  const { replayDelivery, isReplaying, replayResult } = useWebhookStore();
  const [activeTab, setActiveTab] = useState<'payload' | 'headers' | 'hmac' | 'replay'>('payload');
  const [replayUrl, setReplayUrl] = useState('https://httpbin.org/post');
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleReplay = async () => {
    if (!selectedDelivery || !replayUrl.trim()) return;
    await replayDelivery(selectedDelivery.id, replayUrl.trim());
  };

  return (
    <div className="flex-1 p-3 flex flex-col bg-background overflow-hidden min-w-[250px]">
      {selectedDelivery ? (
        <div className="flex-1 flex flex-col gap-3 overflow-hidden">
          {/* Inspector Header & Navigation Tabs */}
          <div className="flex items-center justify-between border-b border-border/60 pb-2 shrink-0">
            <div className="flex items-center gap-1 bg-surface border border-border rounded p-0.5">
              <button
                onClick={() => setActiveTab('payload')}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  activeTab === 'payload'
                    ? 'bg-primary text-primary-foreground font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                Payload Body
              </button>
              <button
                onClick={() => setActiveTab('headers')}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  activeTab === 'headers'
                    ? 'bg-primary text-primary-foreground font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                Headers ({selectedDelivery.headers?.length || 0})
              </button>
              <button
                onClick={() => setActiveTab('hmac')}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  activeTab === 'hmac'
                    ? 'bg-primary text-primary-foreground font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                HMAC Verification
              </button>
              <button
                onClick={() => setActiveTab('replay')}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  activeTab === 'replay'
                    ? 'bg-primary text-primary-foreground font-semibold'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
              >
                Replay Studio
              </button>
            </div>

            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="xs"
                icon={copiedKey === 'payload-copy' ? 'check_line' : 'copy_line'}
                onClick={() => handleCopy(selectedDelivery.payload, 'payload-copy')}
              >
                {copiedKey === 'payload-copy' ? 'Copied' : 'Copy'}
              </Button>
            </div>
          </div>

          {/* Tab: Payload Editor */}
          {activeTab === 'payload' && (
            <div className="flex-1 overflow-hidden border border-border rounded">
              <CodeEditor
                value={selectedDelivery.payload}
                language={
                  selectedDelivery.payload.startsWith('{') || selectedDelivery.payload.startsWith('[')
                    ? 'json'
                    : 'plaintext'
                }
                readOnly
              />
            </div>
          )}

          {/* Tab: Headers Table */}
          {activeTab === 'headers' && (
            <div className="flex-1 overflow-y-auto border border-border rounded bg-surface">
              <table className="w-full text-left font-mono text-xs">
                <thead className="bg-header border-b border-border text-muted-foreground text-2xs sticky top-0">
                  <tr>
                    <th className="px-3 py-1.5 w-1/3">Header Name</th>
                    <th className="px-3 py-1.5">Value</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {Array.isArray(selectedDelivery.headers) &&
                    selectedDelivery.headers.map((h: any, idx: number) => {
                      const key = Array.isArray(h) ? h[0] : h.key || h.name || '';
                      const val = Array.isArray(h) ? h[1] : h.value || '';
                      return (
                        <tr key={idx} className="hover:bg-neutral-subtle/50">
                          <td className="px-3 py-1.5 text-primary font-bold">{key}</td>
                          <td className="px-3 py-1.5 text-foreground break-all">{val}</td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          )}

          {/* Tab: HMAC Details */}
          {activeTab === 'hmac' && (
            <div className="flex-1 overflow-y-auto p-3 bg-surface border border-border rounded space-y-3 font-mono">
              <div className="flex items-center justify-between pb-2 border-b border-border">
                <span className="font-sans font-semibold text-foreground">Signature Status</span>
                <span
                  className={`px-2 py-0.5 rounded text-xs font-bold uppercase border ${
                    selectedDelivery.signatureStatus === 'valid'
                      ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                      : selectedDelivery.signatureStatus === 'invalid'
                      ? 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border-rose-500/30'
                      : 'bg-neutral-subtle text-muted-foreground border-border'
                  }`}
                >
                  {selectedDelivery.signatureStatus}
                </span>
              </div>

              <div className="space-y-1">
                <label className="text-muted-foreground text-2xs block font-sans">
                  Provided Signature Header:
                </label>
                <div className="p-2 bg-background border border-border rounded text-xs break-all text-foreground">
                  {selectedDelivery.providedHmac || '<None Provided>'}
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-muted-foreground text-2xs block font-sans">
                  Computed Expected HMAC:
                </label>
                <div className="p-2 bg-background border border-border rounded text-xs break-all text-foreground">
                  {selectedDelivery.computedHmac || '<Secret not set or no matching algorithm>'}
                </div>
              </div>
            </div>
          )}

          {/* Tab: Replay Studio */}
          {activeTab === 'replay' && (
            <div className="flex-1 flex flex-col gap-3 overflow-hidden">
              <div className="flex items-center gap-2">
                <Input
                  value={replayUrl}
                  onChange={(e) => setReplayUrl(e.target.value)}
                  placeholder="https://your-api.com/webhook-destination"
                  className="flex-1 font-mono text-xs"
                />
                <Button
                  variant="primary"
                  size="sm"
                  icon="repeat_line"
                  onClick={handleReplay}
                  disabled={isReplaying}
                >
                  {isReplaying ? 'Sending...' : 'Replay Delivery'}
                </Button>
              </div>

              {replayResult && (
                <div className="flex-1 flex flex-col border border-border rounded overflow-hidden bg-surface font-mono">
                  <div className="p-2 bg-header border-b border-border flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2">
                      <span
                        className={`px-1.5 py-0.5 rounded text-3xs font-bold border ${
                          replayResult.success
                            ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                            : 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border-rose-500/30'
                        }`}
                      >
                        {replayResult.statusCode} {replayResult.success ? 'OK' : 'FAIL'}
                      </span>
                      <span className="text-muted-foreground text-2xs">{replayResult.durationMs}ms</span>
                    </div>
                    <span className="text-muted-foreground text-2xs">Replay Response Body</span>
                  </div>
                  <div className="flex-1 overflow-hidden">
                    <CodeEditor
                      value={replayResult.responseBody}
                      language={replayResult.responseBody.startsWith('{') ? 'json' : 'plaintext'}
                      readOnly
                    />
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      ) : (
        <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic gap-2">
          <MingCuteIcon name="link_line" size={24} className="opacity-40" />
          <span>Select a webhook delivery log hit from the table to inspect details</span>
        </div>
      )}
    </div>
  );
};
