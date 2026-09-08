import React, { useState } from 'react';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button } from '../../../common/ui';

interface WebhookEndpointsSectionProps {
  onOpenAddModal: () => void;
}

export const WebhookEndpointsSection: React.FC<WebhookEndpointsSectionProps> = ({
  onOpenAddModal,
}) => {
  const {
    listenerConfig,
    toggleServer,
    setPort,
    endpoints,
    deleteEndpoint,
    deliveries,
  } = useWebhookStore();

  const [visibleSecrets, setVisibleSecrets] = useState<Record<string, boolean>>({});
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const toggleSecretVisibility = (id: string) => {
    setVisibleSecrets((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const copyCurlCommand = (epPath: string) => {
    const fullUrl = `http://localhost:${listenerConfig.port}${epPath}`;
    const cmd = `curl -X POST "${fullUrl}" \\\n  -H "Content-Type: application/json" \\\n  -d '{"event":"test_webhook","timestamp":${Date.now()}}'`;
    handleCopy(cmd, `curl-${epPath}`);
  };

  return (
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
          <span>
            Endpoints: <strong className="text-foreground">{endpoints.length}</strong>
          </span>
          <span>
            Captured Deliveries: <strong className="text-foreground">{deliveries.length}</strong>
          </span>
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
            onClick={onOpenAddModal}
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
                    No webhook endpoints registered. Click &quot;Add Endpoint&quot; to create one.
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
  );
};
