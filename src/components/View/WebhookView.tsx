import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  WebhookDelivery,
  WebhookForwardResult,
  WEBHOOK_TEMPLATES,
  WebhookTemplate
} from '@/types/webhook';
import { useWebhooks } from '@/hooks/traffic/useWebhooks';
import { useTraffic } from '@/hooks/traffic';
import { Modal } from '@/components/ui/Modal';
import {
  Webhook,
  Send,
  Plus,
  Trash2,
  RotateCw,
  Copy,
  Check,
  Zap,
  Globe,
  ShieldCheck,
  List,
  ArrowRight,
  Power
} from 'lucide-react';

interface WebhookViewProps {
  onSwitchTab?: (tab: any) => void;
}

export function WebhookView({ onSwitchTab }: WebhookViewProps) {
  const { prefs, updatePrefs } = useTraffic();
  const {
    endpoints,
    deliveries,
    listenerConfig,
    refresh,
    createEndpoint,
    updateEndpoint,
    deleteEndpoint,
    clearDeliveries,
    deleteDelivery,
    forwardDelivery,
    triggerWebhook,
    startListener,
    stopListener,
    calculateSignature
  } = useWebhooks();

  const publicDomain = prefs.webhookPublicDomain || 'https://example.com';
  const setPublicDomain = (domain: string) => {
    updatePrefs({ webhookPublicDomain: domain });
  };

  const [activeSubTab, setActiveSubTab] = useState<'deliveries' | 'endpoints' | 'generator'>('deliveries');
  const [selectedDeliveryId, setSelectedDeliveryId] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  // Forwarding Modal State
  const [forwardModalOpen, setForwardModalOpen] = useState(false);
  const [forwardTargetUrl, setForwardTargetUrl] = useState('http://localhost:3000/api/webhooks');
  const [isForwarding, setIsForwarding] = useState(false);

  // New Endpoint Modal State
  const [endpointModalOpen, setEndpointModalOpen] = useState(false);
  const [newEpName, setNewEpName] = useState('');
  const [newEpSlug, setNewEpSlug] = useState('/hook/stripe');
  const [newEpStatus, setNewEpStatus] = useState(200);
  const [newEpBody, setNewEpBody] = useState('{"status": "received"}');
  const [newEpForward, setNewEpForward] = useState('');

  // Webhook Generator State
  const [selectedTemplate, setSelectedTemplate] = useState<WebhookTemplate>(WEBHOOK_TEMPLATES[0]);
  const [genTargetUrl, setGenTargetUrl] = useState('http://localhost:3000/api/webhooks');
  const [genSecret, setGenSecret] = useState('whsec_test_secret_key_123');
  const [genMethod, setGenMethod] = useState('POST');
  const [genBody, setGenBody] = useState(WEBHOOK_TEMPLATES[0].sampleBody);
  const [calculatedSig, setCalculatedSig] = useState<{ header: string; value: string } | null>(null);
  const [isDispatching, setIsDispatching] = useState(false);
  const [dispatchResult, setDispatchResult] = useState<WebhookForwardResult | null>(null);

  const selectedDelivery = deliveries.find((d) => d.id === selectedDeliveryId) || deliveries[0];

  const handleCopy = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleSendToRepeater = async (delivery?: WebhookDelivery) => {
    const itemToRepeater = delivery || selectedDelivery;
    if (!itemToRepeater) return;

    let parsedHeaders: [string, string][] = [];
    try {
      const hObj = JSON.parse(itemToRepeater.headers);
      parsedHeaders = Object.entries(hObj).map(([k, v]) => [k, String(v)]);
    } catch {
      parsedHeaders = [['Content-Type', 'application/json']];
    }

    try {
      await invoke('create_repeater_item', {
        item: {
          name: `Webhook: ${itemToRepeater.method} ${itemToRepeater.path}`,
          method: itemToRepeater.method,
          url: `http://localhost:3000${itemToRepeater.path}`,
          headers: parsedHeaders,
          body: itemToRepeater.body,
          groupId: null,
          bodyMode: 'json',
          bodyJson: itemToRepeater.body,
        }
      });
      if (onSwitchTab) {
        onSwitchTab('repeater');
      } else {
        window.dispatchEvent(new CustomEvent('switch-tab', { detail: 'repeater' }));
      }
    } catch (err) {
      console.error('Failed to send to Repeater:', err);
    }
  };

  const handleSendGeneratorToRepeater = async () => {
    let headers: [string, string][] = [['Content-Type', 'application/json']];
    if (calculatedSig) {
      headers.push([calculatedSig.header, calculatedSig.value]);
    }

    try {
      await invoke('create_repeater_item', {
        item: {
          name: `Webhook: ${selectedTemplate.name}`,
          method: genMethod,
          url: genTargetUrl,
          headers,
          body: genBody,
          groupId: null,
          bodyMode: 'json',
          bodyJson: genBody,
        }
      });
      if (onSwitchTab) {
        onSwitchTab('repeater');
      } else {
        window.dispatchEvent(new CustomEvent('switch-tab', { detail: 'repeater' }));
      }
    } catch (err) {
      console.error('Failed to send generator to Repeater:', err);
    }
  };

  const handleCalculateSig = async () => {
    if (!genSecret) {
      setCalculatedSig(null);
      return;
    }
    try {
      const res = await calculateSignature(genSecret, genBody, selectedTemplate.provider);
      setCalculatedSig({ header: res.header_name, value: res.header_value });
    } catch (err) {
      console.error('Failed to calculate signature:', err);
    }
  };

  const handleDispatchWebhook = async () => {
    setIsDispatching(true);
    setDispatchResult(null);

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'User-Agent': 'mitm-rust-webhook-agent/1.0',
    };

    if (genSecret) {
      try {
        const sig = await calculateSignature(genSecret, genBody, selectedTemplate.provider);
        headers[sig.header_name] = sig.header_value;
      } catch (e) {
        console.error('Signature calculation failed:', e);
      }
    }

    try {
      const res = await triggerWebhook({
        url: genTargetUrl,
        method: genMethod,
        headers,
        body: genBody,
        signatureSecret: genSecret,
        providerPreset: selectedTemplate.provider,
      });
      setDispatchResult(res);
    } catch (err: any) {
      setDispatchResult({ success: false, error: String(err) });
    } finally {
      setIsDispatching(false);
    }
  };

  const handleOpenNewEndpointModal = () => {
    setNewEpName('');
    setNewEpSlug('/hook/stripe');
    setNewEpStatus(200);
    setNewEpBody('{"status": "ok", "received": true}');
    setNewEpForward('');
    setEndpointModalOpen(true);
  };

  const handleCreateEndpointSubmit = async () => {
    let slug = newEpSlug.trim();
    if (!slug) {
      if (newEpName.trim()) {
        slug = `/hook/${newEpName.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
      } else {
        slug = `/hook/custom-${Date.now().toString().slice(-4)}`;
      }
    }
    if (!slug.startsWith('/')) {
      slug = `/${slug}`;
    }

    try {
      await createEndpoint({
        name: newEpName.trim() || slug,
        pathSlug: slug,
        mockStatus: Number(newEpStatus) || 200,
        mockHeaders: '{"Content-Type": "application/json"}',
        mockBody: newEpBody || '{"status": "ok"}',
        autoForwardUrl: newEpForward.trim() || null,
        isActive: true,
      });
      setEndpointModalOpen(false);
    } catch (err) {
      console.error('Failed to create webhook endpoint:', err);
    }
  };

  const parseHeadersJson = (jsonStr: string): Record<string, string> => {
    try {
      return JSON.parse(jsonStr);
    } catch {
      return {};
    }
  };

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-300 font-mono overflow-hidden">
      {/* Top Controls Header */}
      <header className="flex items-center justify-between px-4 py-2 border-b border-zinc-800 bg-zinc-900/60 shrink-0">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 text-cyan-400 font-bold uppercase tracking-wider text-xs">
            <Webhook className="w-4 h-4 animate-pulse" />
            <span>Webhook_Testing</span>
          </div>

          {/* Sub-tabs */}
          <div className="flex bg-zinc-950 p-1 rounded-md border border-zinc-800 text-xs">
            <button
              onClick={() => setActiveSubTab('deliveries')}
              className={`px-3 py-1 rounded flex items-center gap-1.5 transition-all ${
                activeSubTab === 'deliveries' ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30' : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <List className="w-3.5 h-3.5" />
              <span>Deliveries</span>
              <span className="ml-1 px-1.5 py-0.2 text-[10px] rounded-full bg-zinc-800 text-zinc-300 font-bold">
                {deliveries.length}
              </span>
            </button>

            <button
              onClick={() => setActiveSubTab('endpoints')}
              className={`px-3 py-1 rounded flex items-center gap-1.5 transition-all ${
                activeSubTab === 'endpoints' ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30' : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>Endpoints</span>
              <span className="ml-1 px-1.5 py-0.2 text-[10px] rounded-full bg-zinc-800 text-zinc-300 font-bold">
                {endpoints.length}
              </span>
            </button>

            <button
              onClick={() => setActiveSubTab('generator')}
              className={`px-3 py-1 rounded flex items-center gap-1.5 transition-all ${
                activeSubTab === 'generator' ? 'bg-purple-500/20 text-purple-400 border border-purple-500/30' : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Zap className="w-3.5 h-3.5" />
              <span>Generator / Outgoing</span>
            </button>
          </div>
        </div>

        {/* Server Listener & Cloudflare Tunnel Toggle */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 bg-zinc-950 px-2 py-1 rounded border border-zinc-800 text-xs">
            <Globe className="w-3.5 h-3.5 text-cyan-400" />
            <span className="text-zinc-500 text-[10px] uppercase font-bold">Public Domain:</span>
            <input
              type="text"
              value={publicDomain}
              onChange={(e) => setPublicDomain(e.target.value)}
              placeholder="https://def.my.id"
              className="bg-zinc-900 border border-zinc-800 rounded px-2 py-0.5 text-[11px] text-cyan-300 font-bold focus:outline-none focus:border-cyan-500 w-36"
            />
          </div>

          <div className="flex items-center gap-2 bg-zinc-950 px-3 py-1 rounded border border-zinc-800 text-xs">
            <span className={`w-2 h-2 rounded-full ${listenerConfig.isRunning ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'}`} />
            <span className="text-zinc-400 text-[11px]">Port:</span>
            <span className="text-emerald-400 font-bold">:{listenerConfig.port}</span>
            <button
              onClick={() => (listenerConfig.isRunning ? stopListener() : startListener(listenerConfig.port))}
              className={`ml-2 px-2 py-0.5 rounded text-[10px] uppercase font-bold flex items-center gap-1 border transition-all ${
                listenerConfig.isRunning
                  ? 'bg-rose-500/20 text-rose-400 border-rose-500/30 hover:bg-rose-500/30'
                  : 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30 hover:bg-emerald-500/30'
              }`}
            >
              <Power className="w-3 h-3" />
              {listenerConfig.isRunning ? 'Stop' : 'Start'}
            </button>
          </div>

          <button
            onClick={refresh}
            className="p-1.5 rounded hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 transition-all"
            title="Refresh Webhook Data"
          >
            <RotateCw className="w-4 h-4" />
          </button>
        </div>
      </header>

      {/* Main SubTab Contents */}
      {activeSubTab === 'deliveries' && (
        <div className="flex flex-1 overflow-hidden">
          {/* Left Panel: Deliveries List */}
          <div className="w-1/3 border-r border-zinc-800 flex flex-col bg-zinc-950/50 shrink-0">
            <div className="p-2 border-b border-zinc-800 flex items-center justify-between text-xs">
              <span className="text-zinc-400 uppercase tracking-wider font-semibold">Captured Payloads</span>
              <button
                onClick={clearDeliveries}
                className="text-[11px] text-rose-400 hover:text-rose-300 flex items-center gap-1 hover:underline"
              >
                <Trash2 className="w-3 h-3" /> Clear All
              </button>
            </div>

            <div className="flex-1 overflow-y-auto divide-y divide-zinc-800/50">
              {deliveries.length === 0 ? (
                <div className="p-8 text-center text-zinc-500 text-xs flex flex-col items-center gap-2">
                  <Webhook className="w-8 h-8 text-zinc-700 stroke-1" />
                  <p>No webhooks captured yet.</p>
                  <p className="text-[11px] text-zinc-600">Send an HTTP POST to http://127.0.0.1:{listenerConfig.port}/hook/...</p>
                </div>
              ) : (
                deliveries.map((del) => {
                  const isSelected = selectedDelivery?.id === del.id;
                  const timeStr = new Date(del.timestamp * 1000).toLocaleTimeString();
                  return (
                    <div
                      key={del.id}
                      onClick={() => setSelectedDeliveryId(del.id)}
                      className={`p-3 cursor-pointer transition-all flex flex-col gap-1.5 ${
                        isSelected ? 'bg-zinc-900 border-l-2 border-cyan-500' : 'hover:bg-zinc-900/40'
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="px-1.5 py-0.5 text-[10px] font-bold rounded bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                          {del.method}
                        </span>
                        <span className="text-[10px] text-zinc-500">{timeStr}</span>
                      </div>
                      <div className="font-semibold text-xs text-zinc-200 truncate">{del.path}</div>
                      <div className="flex items-center justify-between text-[10px] text-zinc-400">
                        <span>IP: {del.clientIp || '127.0.0.1'}</span>
                        {del.forwarded && (
                          <span className="px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-400 border border-emerald-500/30">
                            Relayed {del.forwardStatus ? `(${del.forwardStatus})` : ''}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Right Panel: Delivery Inspector */}
          <div className="flex-1 flex flex-col bg-zinc-950 overflow-hidden">
            {selectedDelivery ? (
              <div className="flex flex-col h-full overflow-y-auto p-4 gap-4">
                {/* Header Actions Toolbar */}
                <div className="flex items-center justify-between bg-zinc-900 p-3 rounded-lg border border-zinc-800">
                  <div className="flex items-center gap-3">
                    <span className="px-2 py-1 bg-cyan-500/20 text-cyan-400 font-bold rounded text-xs border border-cyan-500/30">
                      {selectedDelivery.method}
                    </span>
                    <span className="font-bold text-sm text-zinc-100">{selectedDelivery.path}</span>
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleSendToRepeater(selectedDelivery)}
                      className="px-3 py-1.5 bg-purple-500/20 text-purple-400 hover:bg-purple-500/30 border border-purple-500/30 rounded text-xs font-bold flex items-center gap-1.5 transition-all"
                    >
                      <RotateCw className="w-3.5 h-3.5" />
                      Send to Repeater
                    </button>

                    <button
                      onClick={() => setForwardModalOpen(true)}
                      className="px-3 py-1.5 bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/30 border border-cyan-500/30 rounded text-xs font-bold flex items-center gap-1.5 transition-all"
                    >
                      <ArrowRight className="w-3.5 h-3.5" />
                      Forward / Relay
                    </button>

                    <button
                      onClick={() => deleteDelivery(selectedDelivery.id)}
                      className="p-1.5 text-zinc-500 hover:text-rose-400 hover:bg-zinc-800 rounded transition-all"
                      title="Delete Delivery"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Delivery Meta */}
                <div className="grid grid-cols-3 gap-3 text-xs">
                  <div className="bg-zinc-900/60 p-2.5 rounded border border-zinc-800">
                    <span className="text-zinc-500 block text-[10px] uppercase">Timestamp</span>
                    <span className="text-zinc-200 font-semibold">{new Date(selectedDelivery.timestamp * 1000).toLocaleString()}</span>
                  </div>

                  <div className="bg-zinc-900/60 p-2.5 rounded border border-zinc-800">
                    <span className="text-zinc-500 block text-[10px] uppercase">Client IP</span>
                    <span className="text-zinc-200 font-semibold">{selectedDelivery.clientIp || 'Unknown'}</span>
                  </div>

                  <div className="bg-zinc-900/60 p-2.5 rounded border border-zinc-800">
                    <span className="text-zinc-500 block text-[10px] uppercase">Query Parameters</span>
                    <span className="text-zinc-200 font-semibold">{selectedDelivery.queryParams || 'None'}</span>
                  </div>
                </div>

                {/* Headers Section */}
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs uppercase tracking-wider text-zinc-400 font-bold">Request Headers</span>
                  <div className="bg-zinc-900 p-3 rounded-md border border-zinc-800 overflow-x-auto text-xs">
                    {Object.entries(parseHeadersJson(selectedDelivery.headers)).map(([k, v]) => (
                      <div key={k} className="flex gap-2 py-0.5">
                        <span className="text-cyan-400 font-semibold min-w-36">{k}:</span>
                        <span className="text-zinc-300 break-all">{v}</span>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Body Section */}
                <div className="flex flex-col gap-1.5 flex-1">
                  <div className="flex items-center justify-between">
                    <span className="text-xs uppercase tracking-wider text-zinc-400 font-bold">Payload Body</span>
                    <button
                      onClick={() => handleCopy(selectedDelivery.body, 'body')}
                      className="text-xs text-zinc-400 hover:text-zinc-200 flex items-center gap-1"
                    >
                      {copiedId === 'body' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                      {copiedId === 'body' ? 'Copied' : 'Copy'}
                    </button>
                  </div>

                  <div className="bg-zinc-900 p-3 rounded-md border border-zinc-800 font-mono text-xs text-emerald-400 overflow-auto flex-1 whitespace-pre-wrap">
                    {selectedDelivery.body || '<Empty Body>'}
                  </div>
                </div>

                {/* Forward Status if any */}
                {selectedDelivery.forwarded && (
                  <div className="bg-zinc-900/80 p-3 rounded-md border border-emerald-500/30 flex flex-col gap-2">
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-bold text-emerald-400 flex items-center gap-1">
                        <Check className="w-4 h-4" /> Relayed Response Status: {selectedDelivery.forwardStatus}
                      </span>
                    </div>
                    {selectedDelivery.forwardResponseBody && (
                      <pre className="text-[11px] text-zinc-300 bg-zinc-950 p-2 rounded border border-zinc-800 overflow-x-auto max-h-32">
                        {selectedDelivery.forwardResponseBody}
                      </pre>
                    )}
                  </div>
                )}
              </div>
            ) : (
              <div className="flex-1 flex items-center justify-center text-zinc-500 text-xs">
                Select a delivery payload from the left list to inspect details.
              </div>
            )}
          </div>
        </div>
      )}

      {/* Endpoints Management SubTab */}
      {activeSubTab === 'endpoints' && (
        <div className="flex-1 p-6 overflow-y-auto flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-base font-bold text-zinc-100 flex items-center gap-2">
                <Globe className="w-5 h-5 text-cyan-400" />
                Configured Webhook Endpoints
              </h2>
              <p className="text-xs text-zinc-400">Define custom path listeners, mock HTTP response status/bodies, and auto-relay rules.</p>
            </div>

            <button
              onClick={handleOpenNewEndpointModal}
              className="px-4 py-2 bg-cyan-500 hover:bg-cyan-400 text-zinc-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow-md shadow-cyan-500/20 cursor-pointer"
            >
              <Plus className="w-4 h-4" />
              New Endpoint
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-2">
            {endpoints.map((ep) => (
              <div key={ep.id} className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 flex flex-col gap-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="font-bold text-sm text-zinc-100">{ep.name}</span>
                    <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                      http://127.0.0.1:{listenerConfig.port}{ep.pathSlug}
                    </span>
                  </div>

                  <button
                    onClick={() => updateEndpoint({ ...ep, isActive: !ep.isActive })}
                    className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase transition-all ${
                      ep.isActive ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'bg-zinc-800 text-zinc-500'
                    }`}
                  >
                    {ep.isActive ? 'Active' : 'Disabled'}
                  </button>
                </div>

                <div className="grid grid-cols-2 gap-2 text-xs bg-zinc-950 p-2.5 rounded border border-zinc-800/80">
                  <div>
                    <span className="text-zinc-500 block text-[10px] uppercase">Public Domain URL</span>
                    <button
                      onClick={() => handleCopy(`${publicDomain.replace(/\/$/, '')}${ep.pathSlug.startsWith('/') ? ep.pathSlug : '/' + ep.pathSlug}`, ep.id)}
                      className="font-mono text-cyan-400 text-[11px] truncate block hover:underline text-left"
                      title="Click to copy public webhook URL"
                    >
                      {copiedId === ep.id ? 'Copied URL!' : `${publicDomain.replace(/\/$/, '')}${ep.pathSlug.startsWith('/') ? ep.pathSlug : '/' + ep.pathSlug}`}
                    </button>
                  </div>

                  <div>
                    <span className="text-zinc-500 block text-[10px] uppercase">Auto-Forward URL</span>
                    <span className="font-mono text-zinc-300 text-[11px] truncate block">{ep.autoForwardUrl || 'None'}</span>
                  </div>
                </div>

                <div className="flex items-center justify-between pt-2 border-t border-zinc-800/60 text-xs">
                  <span className="text-[10px] text-zinc-500">Created: {new Date(ep.createdAt || Date.now()).toLocaleDateString()}</span>
                  <button
                    onClick={() => deleteEndpoint(ep.id)}
                    className="text-rose-400 hover:text-rose-300 flex items-center gap-1 text-[11px]"
                  >
                    <Trash2 className="w-3.5 h-3.5" /> Delete
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Generator / Outgoing Builder SubTab */}
      {activeSubTab === 'generator' && (
        <div className="flex-1 p-6 overflow-y-auto flex flex-col gap-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-base font-bold text-zinc-100 flex items-center gap-2">
                <Zap className="w-5 h-5 text-purple-400" />
                Outgoing Webhook Builder & Signature Generator
              </h2>
              <p className="text-xs text-zinc-400">
                Construct synthetic webhooks with pre-filled provider templates and auto-calculated HMAC signatures.
              </p>
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Template Selector & Settings */}
            <div className="flex flex-col gap-4 bg-zinc-900 border border-zinc-800 p-4 rounded-lg">
              <span className="text-xs uppercase tracking-wider text-zinc-400 font-bold">1. Select Provider Template</span>
              <div className="flex flex-col gap-2">
                {WEBHOOK_TEMPLATES.map((tmpl) => (
                  <button
                    key={tmpl.id}
                    onClick={() => {
                      setSelectedTemplate(tmpl);
                      setGenBody(tmpl.sampleBody);
                    }}
                    className={`p-3 rounded border text-left text-xs transition-all flex flex-col gap-1 ${
                      selectedTemplate.id === tmpl.id
                        ? 'bg-purple-500/20 border-purple-500/50 text-purple-300'
                        : 'bg-zinc-950 border-zinc-800 text-zinc-400 hover:bg-zinc-800/40'
                    }`}
                  >
                    <span className="font-bold">{tmpl.name}</span>
                    <span className="text-[10px] text-zinc-500">Header: {tmpl.defaultHeader}</span>
                  </button>
                ))}
              </div>

              <span className="text-xs uppercase tracking-wider text-zinc-400 font-bold mt-2">2. HMAC Signing Secret</span>
              <input
                type="text"
                value={genSecret}
                onChange={(e) => setGenSecret(e.target.value)}
                placeholder="whsec_secret_key"
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-xs text-zinc-200 focus:outline-none focus:border-purple-500"
              />

              <button
                onClick={handleCalculateSig}
                className="px-3 py-2 bg-purple-500/20 hover:bg-purple-500/30 text-purple-300 border border-purple-500/30 rounded text-xs font-bold flex items-center justify-center gap-1.5 transition-all"
              >
                <ShieldCheck className="w-4 h-4" />
                Compute HMAC Signature
              </button>

              {calculatedSig && (
                <div className="bg-zinc-950 p-3 rounded border border-purple-500/30 text-xs flex flex-col gap-1">
                  <span className="text-purple-400 font-bold">{calculatedSig.header}:</span>
                  <span className="text-zinc-300 break-all font-mono text-[11px]">{calculatedSig.value}</span>
                </div>
              )}
            </div>

            {/* Target URL & Payload Editor */}
            <div className="lg:col-span-2 flex flex-col gap-4 bg-zinc-900 border border-zinc-800 p-4 rounded-lg">
              <span className="text-xs uppercase tracking-wider text-zinc-400 font-bold">3. Target Destination & Payload</span>

              <div className="flex gap-2">
                <select
                  value={genMethod}
                  onChange={(e) => setGenMethod(e.target.value)}
                  className="bg-zinc-950 border border-zinc-800 text-xs text-cyan-400 font-bold px-3 py-2 rounded focus:outline-none cursor-pointer"
                >
                  <option value="POST" className="bg-zinc-900 text-cyan-400 py-1">POST</option>
                  <option value="PUT" className="bg-zinc-900 text-cyan-400 py-1">PUT</option>
                </select>

                <input
                  type="text"
                  value={genTargetUrl}
                  onChange={(e) => setGenTargetUrl(e.target.value)}
                  placeholder="http://localhost:3000/api/webhooks"
                  className="flex-1 bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-xs text-zinc-200 focus:outline-none focus:border-cyan-500"
                />
              </div>

              <textarea
                value={genBody}
                onChange={(e) => setGenBody(e.target.value)}
                rows={12}
                className="w-full bg-zinc-950 border border-zinc-800 rounded p-3 text-xs text-emerald-400 font-mono focus:outline-none focus:border-cyan-500"
              />

              <div className="flex items-center justify-between pt-2 border-t border-zinc-800">
                <button
                  onClick={handleSendGeneratorToRepeater}
                  className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-xs font-bold flex items-center gap-1.5 transition-all"
                >
                  <RotateCw className="w-4 h-4" />
                  Save to Repeater
                </button>

                <button
                  onClick={handleDispatchWebhook}
                  disabled={isDispatching}
                  className="px-6 py-2 bg-purple-500 hover:bg-purple-400 text-zinc-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow-md shadow-purple-500/20"
                >
                  <Send className="w-4 h-4" />
                  {isDispatching ? 'Dispatching...' : 'Dispatch Webhook Now'}
                </button>
              </div>

              {/* Dispatch Result Response */}
              {dispatchResult && (
                <div className={`p-4 rounded border text-xs flex flex-col gap-2 ${
                  dispatchResult.success ? 'bg-emerald-950/30 border-emerald-500/40' : 'bg-rose-950/30 border-rose-500/40'
                }`}>
                  <div className="flex items-center justify-between font-bold">
                    <span className={dispatchResult.success ? 'text-emerald-400' : 'text-rose-400'}>
                      {dispatchResult.success ? `Dispatch Successful (${dispatchResult.statusCode} OK)` : `Dispatch Failed`}
                    </span>
                  </div>
                  {dispatchResult.responseBody && (
                    <pre className="text-[11px] text-zinc-300 bg-zinc-950 p-2.5 rounded border border-zinc-800 overflow-x-auto max-h-36">
                      {dispatchResult.responseBody}
                    </pre>
                  )}
                  {dispatchResult.error && <p className="text-rose-400">{dispatchResult.error}</p>}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Forwarding Modal */}
      {selectedDelivery && (
        <Modal
          isOpen={forwardModalOpen}
          onClose={() => setForwardModalOpen(false)}
          title="Relay Webhook Payload"
          maxWidth="md"
          footer={
            <div className="flex items-center justify-end gap-2">
              <button
                onClick={() => setForwardModalOpen(false)}
                className="px-3 py-1.5 bg-zinc-800 text-zinc-300 rounded text-xs font-semibold cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={async () => {
                  setIsForwarding(true);
                  await forwardDelivery(selectedDelivery.id, forwardTargetUrl);
                  setIsForwarding(false);
                  setForwardModalOpen(false);
                }}
                className="px-4 py-1.5 bg-cyan-500 hover:bg-cyan-400 text-zinc-950 font-bold rounded text-xs cursor-pointer"
              >
                {isForwarding ? 'Relaying...' : 'Send Relay'}
              </button>
            </div>
          }
        >
          <div className="p-4 flex flex-col gap-4">
            <p className="text-xs text-zinc-400">Re-send captured payload to local development backend URL.</p>

            <input
              type="text"
              value={forwardTargetUrl}
              onChange={(e) => setForwardTargetUrl(e.target.value)}
              placeholder="http://localhost:3000/api/webhooks"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-xs text-zinc-200 focus:outline-none focus:border-cyan-500"
            />
          </div>
        </Modal>
      )}

      {/* New Endpoint Modal */}
      <Modal
        isOpen={endpointModalOpen}
        onClose={() => setEndpointModalOpen(false)}
        title="New Webhook Receiver Endpoint"
        maxWidth="md"
        footer={
          <div className="flex items-center justify-end gap-2">
            <button
              onClick={() => setEndpointModalOpen(false)}
              className="px-3 py-1.5 bg-zinc-800 text-zinc-300 rounded text-xs font-semibold cursor-pointer"
            >
              Cancel
            </button>
            <button
              onClick={handleCreateEndpointSubmit}
              className="px-4 py-1.5 bg-cyan-500 hover:bg-cyan-400 text-zinc-950 font-bold rounded text-xs cursor-pointer"
            >
              Save Endpoint
            </button>
          </div>
        }
      >
        <div className="p-4 flex flex-col gap-3 text-xs">
          <div>
            <label className="text-zinc-400 block mb-1">Name</label>
            <input
              type="text"
              value={newEpName}
              onChange={(e) => setNewEpName(e.target.value)}
              placeholder="Stripe Webhook Listener"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-200 focus:outline-none focus:border-cyan-500"
            />
          </div>

          <div>
            <label className="text-zinc-400 block mb-1">Path Slug</label>
            <input
              type="text"
              value={newEpSlug}
              onChange={(e) => setNewEpSlug(e.target.value)}
              placeholder="/hook/stripe"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-200 focus:outline-none focus:border-cyan-500"
            />
          </div>

          <div>
            <label className="text-zinc-400 block mb-1">Mock HTTP Status</label>
            <input
              type="number"
              value={newEpStatus}
              onChange={(e) => setNewEpStatus(Number(e.target.value))}
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-200 focus:outline-none focus:border-cyan-500"
            />
          </div>

          <div>
            <label className="text-zinc-400 block mb-1">Mock Response Body</label>
            <textarea
              value={newEpBody}
              onChange={(e) => setNewEpBody(e.target.value)}
              rows={3}
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-emerald-400 font-mono focus:outline-none focus:border-cyan-500"
            />
          </div>

          <div>
            <label className="text-zinc-400 block mb-1">Auto-Relay Target URL (Optional)</label>
            <input
              type="text"
              value={newEpForward}
              onChange={(e) => setNewEpForward(e.target.value)}
              placeholder="http://localhost:3000/api/webhooks"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-1.5 text-zinc-200 focus:outline-none focus:border-cyan-500"
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
