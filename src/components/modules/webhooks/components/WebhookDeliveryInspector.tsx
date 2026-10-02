import React, { useState } from 'react';
import type { WebhookDelivery } from '../../../../types';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { CodeEditor } from '../../../common/CodeEditor';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { PaneHeader } from '../../../common/PaneHeader';
import { BodyView, useBodyFormat } from '../../../common/BodyView';
import { KeyValueEditor } from '../../../common/KeyValueEditor';
import { SegmentedControl } from '../../../common/ui';

type Tab = 'body' | 'headers' | 'signature' | 'replay';

const toHeaderPairs = (headers: WebhookDelivery['headers']) =>
  (Array.isArray(headers) ? headers : []).map((h: any) => ({
    key: String(Array.isArray(h) ? h[0] : h.key || h.name || ''),
    value: String(Array.isArray(h) ? h[1] : h.value || ''),
  }));

const SIGNATURE_TEXT: Record<WebhookDelivery['signatureStatus'], { label: string; className: string }> = {
  valid: { label: 'Signature valid', className: 'text-emerald-500' },
  invalid: { label: 'Signature mismatch', className: 'text-rose-500' },
  none: { label: 'No signature checked', className: 'text-muted-foreground' },
};

interface WebhookDeliveryInspectorProps {
  delivery: WebhookDelivery | null;
}

export const WebhookDeliveryInspector: React.FC<WebhookDeliveryInspectorProps> = ({ delivery }) => {
  const { replayDelivery, isReplaying, replayResult } = useWebhookStore();
  const [tab, setTab] = useState<Tab>('body');
  const [replayUrl, setReplayUrl] = useState('https://httpbin.org/post');
  const [copied, setCopied] = useState(false);

  const headers = delivery ? toHeaderPairs(delivery.headers) : [];
  const { format, setFormat, options, mediaInfo } = useBodyFormat(delivery?.payload ?? '', headers);

  if (!delivery) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-2 text-muted-foreground italic">
        <MingCuteIcon name="link_line" size={28} className="opacity-30" />
        <span>Select a delivery to inspect it</span>
      </div>
    );
  }

  const copyPayload = () => {
    navigator.clipboard.writeText(delivery.payload);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const signature = SIGNATURE_TEXT[delivery.signatureStatus];

  return (
    <div className="h-full flex flex-col overflow-hidden">
      <div className="h-10 px-3 bg-header border-b border-border flex items-center gap-2 font-mono shrink-0">
        <span className="font-semibold text-foreground truncate">{delivery.endpointPath}</span>
        <span className="text-2xs text-muted-foreground shrink-0">{new Date(delivery.timestamp).toLocaleString()}</span>
        <span className={`ml-auto text-2xs font-sans shrink-0 ${signature.className}`}>{signature.label}</span>
        <button
          onClick={copyPayload}
          className="flex items-center gap-1 px-1.5 py-0.5 rounded text-2xs font-sans text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer shrink-0"
          title="Copy payload"
        >
          <MingCuteIcon name={copied ? 'check_line' : 'copy_line'} size={12} />
          <span>{copied ? 'Copied' : 'Copy'}</span>
        </button>
      </div>

      <PaneHeader
        tabs={[
          { value: 'body', label: 'Body' },
          { value: 'headers', label: 'Headers', count: headers.length },
          { value: 'signature', label: 'Signature' },
          { value: 'replay', label: 'Replay' },
        ]}
        activeTab={tab}
        onTabChange={(v) => setTab(v as Tab)}
        right={tab === 'body' && <SegmentedControl value={format} onChange={setFormat} options={options} />}
      />

      <div className={`flex-1 min-h-0 ${tab === 'body' ? 'overflow-hidden' : 'overflow-y-auto p-3'}`}>
        {tab === 'body' && (
          <BodyView body={delivery.payload} format={format} mediaInfo={mediaInfo} previewTitle="webhook-payload" emptyMessage="Empty payload" />
        )}

        {tab === 'headers' && <KeyValueEditor items={headers.map((h, i) => ({ id: `wh-${i}`, ...h, enabled: true }))} onChange={() => {}} readOnly />}

        {tab === 'signature' && (
          <div className="space-y-3 font-mono">
            <div className={`font-sans font-semibold ${signature.className}`}>{signature.label}</div>
            <div className="space-y-1">
              <div className="text-3xs uppercase tracking-wider text-muted-foreground font-sans">Signature header received</div>
              <div className="p-2 bg-background border border-border rounded text-2xs break-all text-foreground select-text">
                {delivery.providedHmac || <span className="italic text-muted-foreground">none</span>}
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-3xs uppercase tracking-wider text-muted-foreground font-sans">Expected (computed from the endpoint secret)</div>
              <div className="p-2 bg-background border border-border rounded text-2xs break-all text-foreground select-text">
                {delivery.computedHmac || (
                  <span className="italic text-muted-foreground">no secret set, or no recognised signature header</span>
                )}
              </div>
            </div>
          </div>
        )}

        {tab === 'replay' && (
          <div className="h-full flex flex-col gap-3">
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (replayUrl.trim()) replayDelivery(delivery.id, replayUrl.trim());
              }}
              className="flex items-center gap-2"
            >
              <input
                type="text"
                value={replayUrl}
                onChange={(e) => setReplayUrl(e.target.value)}
                placeholder="https://your-api.example.com/webhook"
                className="flex-1 bg-background border border-border rounded px-2.5 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
              />
              <button
                type="submit"
                disabled={isReplaying || !replayUrl.trim()}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover disabled:opacity-50 cursor-pointer shrink-0"
              >
                <MingCuteIcon name={isReplaying ? 'loading_line' : 'send_plane_line'} size={13} className={isReplaying ? 'animate-spin' : ''} />
                <span>{isReplaying ? 'Sending...' : 'Replay'}</span>
              </button>
            </form>
            <p className="text-2xs text-muted-foreground">Sends this payload and its headers to the URL above.</p>

            {replayResult && (
              <div className="flex-1 min-h-0 flex flex-col border border-border rounded overflow-hidden">
                <div className="px-3 py-1.5 bg-header border-b border-border flex items-center gap-2 font-mono text-2xs">
                  <span className={`font-bold ${replayResult.success ? 'text-emerald-500' : 'text-rose-500'}`}>
                    {replayResult.statusCode || 'ERR'}
                  </span>
                  <span className="text-muted-foreground">{replayResult.durationMs}ms</span>
                </div>
                <div className="flex-1 min-h-0">
                  <CodeEditor
                    value={replayResult.responseBody}
                    language={replayResult.responseBody.trim().startsWith('{') ? 'json' : 'plaintext'}
                    readOnly
                    bare
                  />
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
