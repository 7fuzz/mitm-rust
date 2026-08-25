import React, { useState } from 'react';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { CodeEditor } from '../../common/CodeEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Select } from '../../common/ui';

const HMAC_PROVIDER_OPTIONS = [
  { value: 'github', label: 'GitHub (sha256=...)' },
  { value: 'stripe', label: 'Stripe (t=...,v1=...)' },
  { value: 'raw_sha256', label: 'Raw SHA-256 Hex' },
  { value: 'raw_sha1', label: 'Raw SHA-1 Hex' },
  { value: 'raw_md5', label: 'Raw MD5 Hex' },
] as const;

export const WebhooksView: React.FC = () => {
  const {
    listenerConfig,
    toggleServer,
    setPort,
    endpoints,
    addEndpoint,
    deleteEndpoint,
    deliveries,
    clearDeliveries,
    selectedDeliveryId,
    selectDelivery,
    replayDelivery,
    hmacSecret,
    setHmacSecret,
    hmacBody,
    setHmacBody,
    hmacProvider,
    setHmacProvider,
    computedSignature,
    calculateHmac,
  } = useWebhookStore();

  const [newPath, setNewPath] = useState('');
  const [newName, setNewName] = useState('');
  const [newSecret, setNewSecret] = useState('');
  const [replayUrl, setReplayUrl] = useState('https://webhook.site/test');

  const selectedDelivery = deliveries.find((d) => d.id === selectedDeliveryId) || (deliveries.length > 0 ? deliveries[0] : null);

  const handleCreateEndpoint = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPath.trim()) return;
    await addEndpoint({
      path: newPath.startsWith('/') ? newPath : `/${newPath}`,
      name: newName.trim() || 'Webhook Endpoint',
      secretKey: newSecret.trim() || 'whsec_secret',
    });
    setNewPath('');
    setNewName('');
    setNewSecret('');
  };

  const handleReplay = async () => {
    if (!selectedDelivery || !replayUrl.trim()) return;
    await replayDelivery(selectedDelivery.id, replayUrl.trim());
    alert(`Replayed webhook payload to ${replayUrl}`);
  };

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-y-auto space-y-4 text-xs">
      {/* Top Server Control Panel & Endpoints Manager */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Server Control Panel */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <MingCuteIcon name="link_line" size={16} className="text-primary" />
              <span className="font-semibold text-foreground text-sm">Webhook Receiver</span>
            </div>
            <button
              onClick={toggleServer}
              className={`flex items-center gap-1.5 px-3 py-1 rounded-full font-bold text-xs transition-all shadow-xs ${
                listenerConfig.is_running
                  ? 'bg-emerald-500 text-white ring-4 ring-emerald-500/20'
                  : 'bg-neutral-subtle border border-border text-muted-foreground'
              }`}
            >
              <span className={`w-2 h-2 rounded-full ${listenerConfig.is_running ? 'bg-white' : 'bg-slate-400'}`} />
              <span>{listenerConfig.is_running ? 'RUNNING' : 'STOPPED'}</span>
            </button>
          </div>

          <div className="flex items-center gap-2 font-mono">
            <span className="text-muted-foreground">Listening Port:</span>
            <input
              type="number"
              value={listenerConfig.port}
              onChange={(e) => setPort(parseInt(e.target.value) || 9000)}
              className="w-24 bg-background border border-border rounded px-2 py-1 text-foreground focus:outline-none focus:border-primary text-center font-bold"
            />
          </div>

          <p className="text-[11px] text-muted-foreground">
            Receiver active at <code className="text-primary font-bold">{`http://localhost:${listenerConfig.port}`}</code>
          </p>
        </div>

        {/* Endpoints Table (Spans 2 cols) */}
        <div className="md:col-span-2 bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-foreground text-sm">Registered Webhook Endpoints</span>
          </div>

          <div className="border border-border rounded overflow-hidden">
            <table className="w-full text-left font-mono text-xs">
              <thead>
                <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
                  <th className="px-3 py-1.5 font-sans">Endpoint Path</th>
                  <th className="px-3 py-1.5 font-sans">Name / Description</th>
                  <th className="px-3 py-1.5 font-sans">Secret Key</th>
                  <th className="w-16 px-2 py-1.5 text-center font-sans">Hits</th>
                  <th className="w-12 px-2 py-1.5 text-center font-sans">Action</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border bg-surface">
                {endpoints.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="px-3 py-3 text-center text-muted-foreground italic font-sans">
                      No webhook endpoints registered
                    </td>
                  </tr>
                ) : (
                  endpoints.map((ep) => (
                    <tr key={ep.id} className="hover:bg-neutral-subtle/50">
                      <td className="px-3 py-1.5 font-bold text-primary">{ep.path}</td>
                      <td className="px-3 py-1.5 font-sans">{ep.name}</td>
                      <td className="px-3 py-1.5 text-amber-500 font-bold">{ep.secretKey}</td>
                      <td className="px-2 py-1.5 text-center font-bold">{ep.hitCount}</td>
                      <td className="px-2 py-1.5 text-center">
                        <button onClick={() => deleteEndpoint(ep.id)} className="text-muted-foreground hover:text-rose-500">
                          <MingCuteIcon name="delete_2_line" size={14} />
                        </button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>

          {/* Quick Create Endpoint */}
          <form onSubmit={handleCreateEndpoint} className="flex items-center gap-2 font-mono">
            <input
              type="text"
              value={newPath}
              onChange={(e) => setNewPath(e.target.value)}
              placeholder="/api/v1/custom-webhook"
              className="w-1/3 bg-background border border-border rounded px-2 py-1 text-foreground focus:outline-none focus:border-primary"
            />
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="Endpoint Name..."
              className="w-1/3 bg-background border border-border rounded px-2 py-1 font-sans text-foreground focus:outline-none focus:border-primary"
            />
            <input
              type="text"
              value={newSecret}
              onChange={(e) => setNewSecret(e.target.value)}
              placeholder="Secret Key"
              className="w-1/4 bg-background border border-border rounded px-2 py-1 text-foreground focus:outline-none focus:border-primary"
            />
            <button
              type="submit"
              className="px-3 py-1 bg-primary text-primary-foreground font-sans font-medium rounded hover:bg-primary-hover transition-colors shrink-0"
            >
              Add
            </button>
          </form>
        </div>
      </div>

      {/* Main Delivery Log & Inspector Area */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-foreground text-sm">Delivery Log & Hits</span>
          <button
            onClick={clearDeliveries}
            className="flex items-center gap-1 px-2.5 py-1 rounded bg-neutral-subtle border border-border text-rose-500 hover:bg-rose-500/10 text-xs font-medium"
          >
            <MingCuteIcon name="delete_2_line" size={13} />
            <span>Clear Log</span>
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 h-80">
          {/* Delivery Hits Table */}
          <div className="border border-border rounded overflow-y-auto">
            <table className="w-full text-left font-mono text-xs">
              <thead className="bg-header border-b border-border text-muted-foreground text-[11px] sticky top-0">
                <tr>
                  <th className="px-3 py-1.5 font-sans">Endpoint</th>
                  <th className="px-3 py-1.5 font-sans">Timestamp</th>
                  <th className="px-3 py-1.5 text-center font-sans">HMAC Signature</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border bg-surface">
                {deliveries.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-3 py-6 text-center text-muted-foreground italic font-sans">
                      No webhook deliveries captured yet
                    </td>
                  </tr>
                ) : (
                  deliveries.map((del) => {
                    const isSelected = selectedDelivery?.id === del.id;
                    return (
                      <tr
                        key={del.id}
                        onClick={() => selectDelivery(del.id)}
                        className={`cursor-pointer transition-colors ${
                          isSelected ? 'bg-primary/15 font-semibold text-foreground' : 'hover:bg-neutral-subtle'
                        }`}
                      >
                        <td className="px-3 py-2 text-primary font-bold">{del.endpointPath}</td>
                        <td className="px-3 py-2 text-muted-foreground text-[11px]">
                          {new Date(del.timestamp).toLocaleTimeString()}
                        </td>
                        <td className="px-3 py-2 text-center">
                          <span
                            className={`px-1.5 py-0.5 rounded text-[10px] uppercase font-bold border ${
                              del.signatureStatus === 'valid'
                                ? 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20'
                                : 'bg-rose-500/10 text-rose-500 border-rose-500/20'
                            }`}
                          >
                            {del.signatureStatus}
                          </span>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>

          {/* Delivery Inspector & Replayer */}
          <div className="border border-border rounded p-3 flex flex-col gap-3 bg-background overflow-hidden">
            {selectedDelivery ? (
              <div className="flex-1 flex flex-col gap-2 overflow-hidden">
                {/* Replay Bar */}
                <div className="flex items-center gap-2 font-mono">
                  <input
                    type="text"
                    value={replayUrl}
                    onChange={(e) => setReplayUrl(e.target.value)}
                    placeholder="https://target.com/webhook"
                    className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
                  />
                  <button
                    onClick={handleReplay}
                    className="flex items-center gap-1 px-3 py-1 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover transition-colors shadow-xs shrink-0"
                  >
                    <MingCuteIcon name="repeat_line" size={14} />
                    <span>Replay Hit</span>
                  </button>
                </div>

                {/* Monaco Code Editor Payload */}
                <div className="flex-1 overflow-hidden">
                  <CodeEditor value={selectedDelivery.payload} language="json" readOnly />
                </div>
              </div>
            ) : (
              <div className="h-full flex items-center justify-center text-muted-foreground italic">
                Select a webhook delivery log hit from the table
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Built-in HMAC Calculator Utility */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
        <div className="flex items-center gap-2">
          <MingCuteIcon name="key_line" size={16} className="text-primary" />
          <span className="font-semibold text-foreground text-sm">Built-in HMAC Signature Calculator</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 font-mono">
          <div>
            <label className="block text-muted-foreground mb-1 font-sans">Provider Type:</label>
            <Select
              value={hmacProvider}
              onChange={(e) => setHmacProvider(e.target.value as any)}
              options={HMAC_PROVIDER_OPTIONS}
              className="w-full"
            />
          </div>

          <div className="md:col-span-2">
            <label className="block text-muted-foreground mb-1 font-sans">Secret Key & Action:</label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={hmacSecret}
                onChange={(e) => setHmacSecret(e.target.value)}
                placeholder="Secret Key..."
                className="w-full bg-background border border-border rounded px-2.5 py-1.5 text-foreground focus:outline-none focus:border-primary"
              />
              <button
                onClick={calculateHmac}
                className="px-4 py-1.5 bg-primary text-primary-foreground font-sans font-medium rounded hover:bg-primary-hover shadow-xs shrink-0"
              >
                Calculate
              </button>
            </div>
          </div>
        </div>

        <div>
          <label className="block text-muted-foreground mb-1 font-sans">Payload Body:</label>
          <textarea
            rows={3}
            value={hmacBody}
            onChange={(e) => setHmacBody(e.target.value)}
            placeholder="JSON or raw text body payload for HMAC calculation..."
            className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
          />
        </div>

        {computedSignature && (
          <div className="p-2.5 bg-background border border-border rounded font-mono text-xs space-y-1">
            <div className="text-muted-foreground">Header Name: <strong className="text-primary">{computedSignature.header_name}</strong></div>
            <div className="text-foreground font-bold break-all">Header Value: {computedSignature.header_value}</div>
          </div>
        )}
      </div>
    </div>
  );
};
