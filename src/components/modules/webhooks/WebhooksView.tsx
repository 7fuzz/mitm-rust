import React, { useState, useEffect } from 'react';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { CodeEditor } from '../../common/CodeEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button, Input, Select, Dialog } from '../../common/ui';

const HMAC_PROVIDER_OPTIONS = [
  { value: 'github', label: 'GitHub (X-Hub-Signature-256 / sha256=...)' },
  { value: 'stripe', label: 'Stripe (Stripe-Signature / t=...,v1=...)' },
  { value: 'shopify', label: 'Shopify (X-Shopify-Hmac-SHA256 / Base64)' },
  { value: 'raw_sha256', label: 'Generic SHA-256 (Hex)' },
  { value: 'raw_sha1', label: 'Generic SHA-1 (Hex)' },
  { value: 'raw_sha512', label: 'Generic SHA-512 (Hex)' },
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
    isReplaying,
    replayResult,
    hmacSecret,
    setHmacSecret,
    hmacBody,
    setHmacBody,
    hmacProvider,
    setHmacProvider,
    computedSignature,
    calculateHmac,
    initialize,
  } = useWebhookStore();

  const [newPath, setNewPath] = useState('');
  const [newName, setNewName] = useState('');
  const [newSecret, setNewSecret] = useState('');
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [visibleSecrets, setVisibleSecrets] = useState<Record<string, boolean>>({});
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState<'payload' | 'headers' | 'hmac' | 'replay'>('payload');
  const [replayUrl, setReplayUrl] = useState('https://httpbin.org/post');

  // Resizable delivery list & inspector
  const [deliveryListPercent, setDeliveryListPercent] = useState<number>(42);
  const deliveryAreaRef = React.useRef<HTMLDivElement>(null);
  const isResizingDelivery = React.useRef(false);

  const handleMouseDownDeliverySplit = (e: React.MouseEvent) => {
    e.preventDefault();
    isResizingDelivery.current = true;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizingDelivery.current || !deliveryAreaRef.current) return;
      const rect = deliveryAreaRef.current.getBoundingClientRect();
      const relativeX = moveEvent.clientX - rect.left;
      const newPercent = (relativeX / rect.width) * 100;
      setDeliveryListPercent(Math.min(Math.max(newPercent, 20), 80));
    };

    const handleMouseUp = () => {
      isResizingDelivery.current = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  useEffect(() => {
    initialize();
  }, [initialize]);

  const selectedDelivery =
    deliveries.find((d) => d.id === selectedDeliveryId) ||
    (deliveries.length > 0 ? deliveries[0] : null);

  const filteredDeliveries = deliveries.filter((d) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      d.endpointPath.toLowerCase().includes(q) ||
      d.payload.toLowerCase().includes(q) ||
      d.signatureStatus.toLowerCase().includes(q)
    );
  });

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleCreateEndpoint = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPath.trim()) return;
    const formattedPath = newPath.trim().startsWith('/') ? newPath.trim() : `/${newPath.trim()}`;
    await addEndpoint({
      path: formattedPath,
      name: newName.trim() || 'Webhook Endpoint',
      secretKey: newSecret.trim() || `whsec_${Math.random().toString(36).substring(2, 10)}`,
    });
    setNewPath('');
    setNewName('');
    setNewSecret('');
    setIsAddModalOpen(false);
  };

  const toggleSecretVisibility = (id: string) => {
    setVisibleSecrets((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleReplay = async () => {
    if (!selectedDelivery || !replayUrl.trim()) return;
    await replayDelivery(selectedDelivery.id, replayUrl.trim());
  };

  const copyCurlCommand = (epPath: string) => {
    const fullUrl = `http://localhost:${listenerConfig.port}${epPath}`;
    const cmd = `curl -X POST "${fullUrl}" \\\n  -H "Content-Type: application/json" \\\n  -d '{"event":"test_webhook","timestamp":${Date.now()}}'`;
    handleCopy(cmd, `curl-${epPath}`);
  };

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-y-auto space-y-4 text-xs font-sans">
      {/* Top Header Banner: Server Controller & Quick Statistics */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Webhook Receiver Status Panel */}
        <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="w-7 h-7 rounded-md bg-primary/10 flex items-center justify-center text-primary">
                <MingCuteIcon name="link_line" size={16} />
              </div>
              <div>
                <span className="font-semibold text-foreground text-sm block">Webhook Receiver</span>
                <span className="text-[11px] text-muted-foreground">Embedded HTTP Listener</span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <span
                className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold border ${
                  listenerConfig.is_running
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                    : 'bg-neutral-subtle text-muted-foreground border-border'
                }`}
              >
                <span
                  className={`w-2 h-2 rounded-full ${
                    listenerConfig.is_running ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'
                  }`}
                />
                <span>{listenerConfig.is_running ? 'Listening' : 'Stopped'}</span>
              </span>

              <Button
                variant={listenerConfig.is_running ? 'secondary' : 'primary'}
                size="xs"
                onClick={toggleServer}
                className="font-medium"
                icon={listenerConfig.is_running ? 'power_line' : 'play_line'}
              >
                {listenerConfig.is_running ? 'Disable' : 'Enable'}
              </Button>
            </div>
          </div>

          <div className="flex items-center gap-2 font-mono bg-background/50 border border-border/60 rounded px-2.5 py-1.5">
            <span className="text-muted-foreground text-[11px]">Port:</span>
            <input
              type="number"
              value={listenerConfig.port}
              onChange={(e) => setPort(parseInt(e.target.value) || 9000)}
              className="w-20 bg-background border border-border rounded px-2 py-0.5 text-foreground focus:outline-none focus:border-primary text-center font-bold text-xs"
            />
            <span className="text-muted-foreground text-[11px] ml-auto">
              <code>http://127.0.0.1:{listenerConfig.port}</code>
            </span>
          </div>

          <div className="flex items-center justify-between text-[11px] text-muted-foreground pt-1 border-t border-border/40">
            <span>Endpoints: <strong className="text-foreground">{endpoints.length}</strong></span>
            <span>Captured Deliveries: <strong className="text-foreground">{deliveries.length}</strong></span>
          </div>
        </div>

        {/* Registered Webhook Endpoints (Spans 2 cols) */}
        <div className="lg:col-span-2 bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs flex flex-col justify-between">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <MingCuteIcon name="route_line" size={16} className="text-primary" />
              <span className="font-semibold text-foreground text-sm">Active Endpoints</span>
            </div>
            <Button
              variant="primary"
              size="xs"
              icon="add_line"
              onClick={() => setIsAddModalOpen(true)}
            >
              Add Endpoint
            </Button>
          </div>

          <div className="border border-border rounded overflow-hidden max-h-44 overflow-y-auto">
            <table className="w-full text-left font-mono text-xs">
              <thead>
                <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
                  <th className="px-3 py-1.5 font-sans">Path & Local URL</th>
                  <th className="px-3 py-1.5 font-sans">Name</th>
                  <th className="px-3 py-1.5 font-sans">Secret Key</th>
                  <th className="w-16 px-2 py-1.5 text-center font-sans">Hits</th>
                  <th className="w-20 px-2 py-1.5 text-center font-sans">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border bg-surface">
                {endpoints.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="px-3 py-4 text-center text-muted-foreground italic font-sans">
                      No webhook endpoints registered. Click "Add Endpoint" to create one.
                    </td>
                  </tr>
                ) : (
                  endpoints.map((ep) => {
                    const fullUrl = `http://localhost:${listenerConfig.port}${ep.path}`;
                    const isSecretVisible = visibleSecrets[ep.id];
                    return (
                      <tr key={ep.id} className="hover:bg-neutral-subtle/50 transition-colors">
                        <td className="px-3 py-1.5">
                          <div className="flex items-center gap-1.5">
                            <span className="font-bold text-primary">{ep.path}</span>
                            <button
                              onClick={() => handleCopy(fullUrl, `url-${ep.id}`)}
                              className="text-muted-foreground hover:text-primary transition-colors p-0.5"
                              title="Copy Full Local URL"
                            >
                              <MingCuteIcon
                                name={copiedKey === `url-${ep.id}` ? 'check_line' : 'copy_line'}
                                size={12}
                              />
                            </button>
                          </div>
                        </td>
                        <td className="px-3 py-1.5 font-sans text-muted-foreground text-[11px]">{ep.name}</td>
                        <td className="px-3 py-1.5">
                          <div className="flex items-center gap-1.5">
                            <span className="text-amber-500 font-bold">
                              {isSecretVisible ? ep.secretKey : '••••••••••••'}
                            </span>
                            <button
                              onClick={() => toggleSecretVisibility(ep.id)}
                              className="text-muted-foreground hover:text-foreground p-0.5"
                              title={isSecretVisible ? 'Hide Secret' : 'Reveal Secret'}
                            >
                              <MingCuteIcon name={isSecretVisible ? 'eye_close_line' : 'eye_line'} size={12} />
                            </button>
                            <button
                              onClick={() => handleCopy(ep.secretKey, `sec-${ep.id}`)}
                              className="text-muted-foreground hover:text-primary p-0.5"
                              title="Copy Secret Key"
                            >
                              <MingCuteIcon
                                name={copiedKey === `sec-${ep.id}` ? 'check_line' : 'copy_line'}
                                size={12}
                              />
                            </button>
                          </div>
                        </td>
                        <td className="px-2 py-1.5 text-center">
                          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-neutral-subtle text-foreground border border-border">
                            {ep.hitCount}
                          </span>
                        </td>
                        <td className="px-2 py-1.5 text-center">
                          <div className="flex items-center justify-center gap-1">
                            <button
                              onClick={() => copyCurlCommand(ep.path)}
                              className="text-muted-foreground hover:text-primary transition-colors p-1"
                              title="Copy cURL Command"
                            >
                              <MingCuteIcon
                                name={copiedKey === `curl-${ep.path}` ? 'check_line' : 'terminal_box_line'}
                                size={14}
                              />
                            </button>
                            <button
                              onClick={() => deleteEndpoint(ep.id)}
                              className="text-muted-foreground hover:text-rose-500 transition-colors p-1"
                              title="Delete Endpoint"
                            >
                              <MingCuteIcon name="delete_2_line" size={14} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Main Delivery Log & Inspector Area */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="history_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">Delivery Logs & Captured Payloads</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-56">
              <Input
                placeholder="Filter hits by path or payload..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                leftIcon="search_line"
              />
            </div>
            <Button
              variant="destructive"
              size="xs"
              icon="delete_2_line"
              onClick={clearDeliveries}
              disabled={deliveries.length === 0}
            >
              Clear Log
            </Button>
          </div>
        </div>

        <div ref={deliveryAreaRef} className="flex h-[420px] overflow-hidden border border-border rounded">
          {/* Delivery Hits Table */}
          <div
            style={{ width: `${deliveryListPercent}%` }}
            className="border-r border-border overflow-y-auto bg-background shrink-0 min-w-[200px]"
          >
            <table className="w-full text-left font-mono text-xs">
              <thead className="bg-header border-b border-border text-muted-foreground text-[11px] sticky top-0 z-10">
                <tr>
                  <th className="px-3 py-1.5 font-sans">Endpoint</th>
                  <th className="px-3 py-1.5 font-sans">Time</th>
                  <th className="px-3 py-1.5 text-center font-sans">HMAC Signature</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {filteredDeliveries.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-3 py-12 text-center text-muted-foreground italic font-sans">
                      {deliveries.length === 0
                        ? 'No webhook deliveries captured yet. Send HTTP requests to your registered endpoint.'
                        : 'No deliveries match your search filter.'}
                    </td>
                  </tr>
                ) : (
                  filteredDeliveries.map((del) => {
                    const isSelected = selectedDelivery?.id === del.id;
                    return (
                      <tr
                        key={del.id}
                        onClick={() => selectDelivery(del.id)}
                        className={`cursor-pointer transition-colors ${
                          isSelected
                            ? 'bg-primary/15 font-semibold text-foreground ring-1 ring-inset ring-primary'
                            : 'hover:bg-neutral-subtle'
                        }`}
                      >
                        <td className="px-3 py-2 text-primary font-bold truncate max-w-[140px]" title={del.endpointPath}>
                          {del.endpointPath}
                        </td>
                        <td className="px-3 py-2 text-muted-foreground text-[11px]">
                          {new Date(del.timestamp).toLocaleTimeString()}
                        </td>
                        <td className="px-3 py-2 text-center">
                          <span
                            className={`px-1.5 py-0.5 rounded text-[10px] font-bold border ${
                              del.signatureStatus === 'valid'
                                ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                                : del.signatureStatus === 'invalid'
                                ? 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border-rose-500/30'
                                : 'bg-neutral-subtle text-muted-foreground border-border'
                            }`}
                          >
                            {del.signatureStatus.toUpperCase()}
                          </span>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>

          {/* Draggable Resizer Handle */}
          <div
            onMouseDown={handleMouseDownDeliverySplit}
            className="w-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-col-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
            title="Drag to resize panels"
          >
            <div className="w-0.5 h-6 rounded bg-muted-foreground/40 group-hover:bg-primary" />
          </div>

          {/* Delivery Inspector & Replayer */}
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
                      language={selectedDelivery.payload.startsWith('{') || selectedDelivery.payload.startsWith('[') ? 'json' : 'plaintext'}
                      readOnly
                    />
                  </div>
                )}

                {/* Tab: Headers Table */}
                {activeTab === 'headers' && (
                  <div className="flex-1 overflow-y-auto border border-border rounded bg-surface">
                    <table className="w-full text-left font-mono text-xs">
                      <thead className="bg-header border-b border-border text-muted-foreground text-[11px] sticky top-0">
                        <tr>
                          <th className="px-3 py-1.5 w-1/3">Header Name</th>
                          <th className="px-3 py-1.5">Value</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-border">
                        {Array.isArray(selectedDelivery.headers) &&
                          selectedDelivery.headers.map((h: any, idx: number) => {
                            const key = Array.isArray(h) ? h[0] : (h.key || h.name || '');
                            const val = Array.isArray(h) ? h[1] : (h.value || '');
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
                      <label className="text-muted-foreground text-[11px] block font-sans">Provided Signature Header:</label>
                      <div className="p-2 bg-background border border-border rounded text-xs break-all text-foreground">
                        {selectedDelivery.providedHmac || '<None Provided>'}
                      </div>
                    </div>

                    <div className="space-y-1">
                      <label className="text-muted-foreground text-[11px] block font-sans">Computed Expected HMAC:</label>
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
                              className={`px-1.5 py-0.5 rounded text-[10px] font-bold border ${
                                replayResult.success
                                  ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/30'
                                  : 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border-rose-500/30'
                              }`}
                            >
                              {replayResult.statusCode} {replayResult.success ? 'OK' : 'FAIL'}
                            </span>
                            <span className="text-muted-foreground text-[11px]">{replayResult.durationMs}ms</span>
                          </div>
                          <span className="text-muted-foreground text-[11px]">Replay Response Body</span>
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
        </div>
      </div>

      {/* Built-in HMAC Signature Sandbox & Generator */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="key_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">HMAC Signature Sandbox & Generator</span>
          </div>
          <Button variant="primary" size="xs" icon="key_line" onClick={calculateHmac}>
            Calculate Signature
          </Button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div>
            <label className="block text-muted-foreground mb-1 text-[11px]">Provider Format:</label>
            <Select
              value={hmacProvider}
              onChange={(e) => setHmacProvider(e.target.value as any)}
              options={HMAC_PROVIDER_OPTIONS}
              className="w-full"
            />
          </div>

          <div className="md:col-span-2">
            <label className="block text-muted-foreground mb-1 text-[11px]">Signing Secret Key:</label>
            <Input
              value={hmacSecret}
              onChange={(e) => setHmacSecret(e.target.value)}
              placeholder="e.g. whsec_secret_key"
              className="font-mono"
            />
          </div>
        </div>

        <div>
          <label className="block text-muted-foreground mb-1 text-[11px]">Payload Body:</label>
          <textarea
            rows={3}
            value={hmacBody}
            onChange={(e) => setHmacBody(e.target.value)}
            placeholder="JSON or raw webhook payload body..."
            className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
          />
        </div>

        {computedSignature && (
          <div className="p-3 bg-background border border-border rounded font-mono text-xs space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">
                Header: <strong className="text-primary">{computedSignature.header_name}</strong>
              </span>
              <Button
                variant="ghost"
                size="xs"
                icon={copiedKey === 'hmac-sig' ? 'check_line' : 'copy_line'}
                onClick={() =>
                  handleCopy(`${computedSignature.header_name}: ${computedSignature.header_value}`, 'hmac-sig')
                }
              >
                {copiedKey === 'hmac-sig' ? 'Copied' : 'Copy Header'}
              </Button>
            </div>
            <div className="p-2 bg-surface border border-border rounded text-foreground font-bold break-all text-xs">
              {computedSignature.header_value}
            </div>
          </div>
        )}
      </div>

      {/* Add Webhook Endpoint Modal Dialog */}
      <Dialog
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        title="Register New Webhook Endpoint"
        footer={
          <div className="flex items-center justify-end gap-2">
            <Button variant="secondary" onClick={() => setIsAddModalOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={handleCreateEndpoint}>
              Register Endpoint
            </Button>
          </div>
        }
      >
        <form onSubmit={handleCreateEndpoint} className="space-y-3 font-sans">
          <div>
            <label className="block text-foreground font-medium mb-1">Path Slug</label>
            <Input
              value={newPath}
              onChange={(e) => setNewPath(e.target.value)}
              placeholder="/api/v1/github-webhook"
              className="font-mono"
              required
            />
            <p className="text-[11px] text-muted-foreground mt-1">
              Accessible locally at <code>http://localhost:{listenerConfig.port}{newPath.startsWith('/') ? newPath : `/${newPath}`}</code>
            </p>
          </div>

          <div>
            <label className="block text-foreground font-medium mb-1">Endpoint Name / Service</label>
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. GitHub Push Event Receiver"
            />
          </div>

          <div>
            <label className="block text-foreground font-medium mb-1">HMAC Secret Key (Optional)</label>
            <Input
              value={newSecret}
              onChange={(e) => setNewSecret(e.target.value)}
              placeholder="whsec_custom_secret"
              className="font-mono"
            />
            <p className="text-[11px] text-muted-foreground mt-1">
              Used to automatically verify incoming HMAC signature headers.
            </p>
          </div>
        </form>
      </Dialog>
    </div>
  );
};

