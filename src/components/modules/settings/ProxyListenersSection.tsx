import React, { useCallback, useEffect, useId, useState } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Switch } from '../../common/ui';
import type { ListenerConfig } from '../../../types';
import { isTauriAvailable } from '../../../services/tauri/ipc';
import { listHostIps, type HostIp } from '../../../services/tauri/bridge';
import { SettingsGroup, SettingsSection } from './SettingsSection';

const WILDCARD_HOSTS = ['0.0.0.0', '::'];

const splitAddress = (address: string): { host: string; port: string } => {
  const idx = address.lastIndexOf(':');
  if (idx === -1) return { host: '', port: address };
  return { host: address.slice(0, idx).replace(/^\[|\]$/g, ''), port: address.slice(idx + 1) };
};

const normalizeHost = (host: string): string => {
  const h = host.trim();
  if (!h) return '0.0.0.0';
  if (h.toLowerCase() === 'localhost') return '127.0.0.1';
  return h;
};

const joinAddress = (host: string, port: string): string => {
  const h = normalizeHost(host);
  return h.includes(':') ? `[${h}]:${port.trim()}` : `${h}:${port.trim()}`;
};

const isValidPort = (port: string): boolean => {
  const p = Number(port.trim());
  return /^\d+$/.test(port.trim()) && p >= 1 && p <= 65535;
};

interface ConflictResult {
  error: string | null;
  displaced: ListenerConfig[];
}

/**
 * Mirrors the backend rule: a wildcard (0.0.0.0 / ::) listener owns its port exclusively.
 * Specific IPs on a wildcard port are rejected; a new wildcard displaces specific IPs (after confirmation).
 */
const checkConflict = (
  host: string,
  port: string,
  listeners: ListenerConfig[],
  ignoreId?: number
): ConflictResult => {
  if (!isValidPort(port)) return { error: 'Port must be a number between 1 and 65535.', displaced: [] };

  const candHost = normalizeHost(host);
  const candWildcard = WILDCARD_HOSTS.includes(candHost);
  const displaced: ListenerConfig[] = [];

  for (const l of listeners) {
    if (l.id === ignoreId) continue;
    const other = splitAddress(l.address);
    if (other.port !== port.trim()) continue;
    const otherHost = normalizeHost(other.host);

    if (otherHost === candHost) {
      return { error: `${joinAddress(host, port)} is already used by '${l.label}'.`, displaced: [] };
    }
    if (WILDCARD_HOSTS.includes(otherHost)) {
      return {
        error: `Port ${port.trim()} is already bound to all interfaces by '${l.label}' (${l.address}).`,
        displaced: [],
      };
    }
    if (candWildcard) displaced.push(l);
  }

  if (displaced.some((l) => l.id === 0)) {
    return {
      error: `Binding all interfaces on port ${port.trim()} would replace listener #0. Change #0's address first.`,
      displaced: [],
    };
  }
  return { error: null, displaced };
};

const ConflictNotice: React.FC<{ conflict: ConflictResult }> = ({ conflict }) => {
  if (conflict.error) {
    return (
      <div className="flex items-start gap-1.5 text-3xs text-rose-400 font-mono bg-rose-500/10 border border-rose-500/20 p-1.5 rounded animate-fade-in">
        <MingCuteIcon name="alert_line" size={12} className="shrink-0 mt-px" />
        <span>{conflict.error}</span>
      </div>
    );
  }
  if (conflict.displaced.length > 0) {
    return (
      <div className="flex items-start gap-1.5 text-3xs text-amber-400 font-mono bg-amber-500/10 border border-amber-500/20 p-1.5 rounded animate-fade-in">
        <MingCuteIcon name="alert_line" size={12} className="shrink-0 mt-px" />
        <span>
          0.0.0.0 covers every interface on this port, so these listeners will be removed:{' '}
          {conflict.displaced.map((l) => `${l.label} (${l.address})`).join(', ')}
        </span>
      </div>
    );
  }
  return null;
};

const inputClass =
  'bg-background border border-border rounded px-2.5 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary min-w-0';

interface ListenerFormProps {
  initial?: { label: string; host: string; port: string };
  listeners: ListenerConfig[];
  ignoreId?: number;
  submitLabel: string;
  submitIcon: string;
  onSubmit: (label: string, address: string, replaceConflicts: boolean) => Promise<void>;
  onCancel?: () => void;
  /** This machine's IPs, offered as host suggestions */
  hostIps: HostIp[];
  /** Sets the host field from outside; the nonce makes the same host apply again */
  hostPreset?: { host: string; nonce: number };
}

const ListenerForm: React.FC<ListenerFormProps> = ({
  initial,
  listeners,
  ignoreId,
  submitLabel,
  submitIcon,
  onSubmit,
  onCancel,
  hostIps,
  hostPreset,
}) => {
  const [label, setLabel] = useState(initial?.label ?? '');
  const [host, setHost] = useState(initial?.host ?? '');
  const [port, setPort] = useState(initial?.port ?? '');
  const [busy, setBusy] = useState(false);
  const hostListId = useId();

  useEffect(() => {
    if (hostPreset) setHost(hostPreset.host);
  }, [hostPreset]);

  const touched = label.trim() !== '' || host.trim() !== '' || port.trim() !== '';
  const conflict: ConflictResult = port.trim()
    ? checkConflict(host, port, listeners, ignoreId)
    : { error: null, displaced: [] };
  const replacing = conflict.displaced.length > 0;
  const canSubmit = !busy && label.trim() !== '' && isValidPort(port) && !conflict.error;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    try {
      await onSubmit(label.trim(), joinAddress(host, port), replacing);
      if (!initial) {
        setLabel('');
        setHost('');
        setPort('');
      }
    } catch {
      // Error is surfaced by the parent's feedback banner; keep the inputs for correction
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-2">
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder="Label"
          className={`${inputClass} w-1/4`}
        />
        <input
          type="text"
          value={host}
          onChange={(e) => setHost(e.target.value)}
          placeholder="Host (blank = 0.0.0.0)"
          title="IP address to bind. 0.0.0.0 = all interfaces, 127.0.0.1 / localhost = this machine only"
          list={hostListId}
          className={`${inputClass} flex-1`}
        />
        <datalist id={hostListId}>
          <option value="0.0.0.0">All interfaces</option>
          {hostIps.map((h) => (
            <option key={`${h.interface}-${h.ip}`} value={h.ip}>
              {h.interface}
            </option>
          ))}
        </datalist>
        <span className="text-muted-foreground font-mono">:</span>
        <input
          type="text"
          inputMode="numeric"
          value={port}
          onChange={(e) => setPort(e.target.value.replace(/[^\d]/g, ''))}
          placeholder="Port"
          className={`${inputClass} w-20`}
        />
        <button
          type="submit"
          disabled={!canSubmit}
          className={`flex items-center gap-1 px-3 py-1.5 font-semibold rounded transition-colors shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed text-xs shrink-0 ${
            replacing
              ? 'bg-amber-500 text-black hover:bg-amber-400'
              : 'bg-primary text-primary-foreground hover:bg-primary-hover'
          }`}
        >
          <MingCuteIcon
            name={busy ? 'loading_line' : submitIcon}
            size={12}
            className={busy ? 'animate-spin' : ''}
          />
          <span>{replacing ? `Replace & ${submitLabel}` : submitLabel}</span>
        </button>
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="text-muted-foreground hover:text-foreground text-2xs px-1.5 cursor-pointer shrink-0"
          >
            Cancel
          </button>
        )}
      </div>
      {touched && <ConflictNotice conflict={conflict} />}
    </form>
  );
};

const statusOf = (l: ListenerConfig): { dot: string; text: string } => {
  if (!l.enabled) return { dot: 'bg-muted-foreground/40', text: 'Disabled' };
  if (l.error) return { dot: 'bg-rose-400', text: 'Failed' };
  if (l.running) return { dot: 'bg-emerald-400', text: 'Listening' };
  return { dot: 'bg-amber-400', text: 'Idle (proxy off)' };
};

export const ProxyListenersSection: React.FC = () => {
  const { listeners, fetchListeners, addListener, removeListener, updateListener, setListenerEnabled } =
    useProxyStore();

  const [editingId, setEditingId] = useState<number | null>(null);
  const [feedback, setFeedback] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [hostIps, setHostIps] = useState<HostIp[]>([]);
  const [hostIpsError, setHostIpsError] = useState<string | null>(null);
  const [showAllIps, setShowAllIps] = useState(false);
  const [hostPreset, setHostPreset] = useState<{ host: string; nonce: number }>();
  const [copiedIp, setCopiedIp] = useState<string | null>(null);

  useEffect(() => {
    fetchListeners();
  }, [fetchListeners]);

  const loadHostIps = useCallback(async () => {
    if (!isTauriAvailable()) return;
    try {
      setHostIps(await listHostIps());
      setHostIpsError(null);
    } catch (err) {
      setHostIpsError(String(err));
    }
  }, []);

  useEffect(() => {
    loadHostIps();
  }, [loadHostIps]);

  const copyIp = (ip: string) => {
    navigator.clipboard.writeText(ip);
    setCopiedIp(ip);
    setTimeout(() => setCopiedIp((c) => (c === ip ? null : c)), 1500);
  };

  // IPv6 and link-local addresses are rarely what a device should point at; tuck them away
  const isSecondaryIp = (h: HostIp) => h.isIpv6 || h.isLinkLocal;
  const shownIps = showAllIps ? hostIps : hostIps.filter((h) => !isSecondaryIp(h));
  const hiddenIpCount = hostIps.length - hostIps.filter((h) => !isSecondaryIp(h)).length;

  const listenersOn = (ip: string) =>
    listeners.filter((l) => normalizeHost(splitAddress(l.address).host) === ip);
  const wildcardListeners = listeners.filter((l) => WILDCARD_HOSTS.includes(normalizeHost(splitAddress(l.address).host)));

  const flash = (type: 'success' | 'error', message: string) => {
    setFeedback({ type, message });
    if (type === 'success') setTimeout(() => setFeedback(null), 4000);
  };

  const run = async (action: () => Promise<void>, success: string) => {
    try {
      await action();
      flash('success', success);
    } catch (err: any) {
      console.error('Listener operation failed:', err);
      flash('error', String(err?.message || err));
      throw err;
    }
  };

  const runningCount = listeners.filter((l) => l.running).length;

  return (
    <SettingsSection
      title="Proxy listeners"
      description={
        <>
          Each listener binds a host and port, and traffic it captures is tagged with its label in History.{' '}
          <code className="font-mono text-foreground">0.0.0.0</code> takes over the whole port (all interfaces); use
          specific IPs to tell clients apart on the same port.
        </>
      }
      aside={
        <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-3xs font-mono bg-primary/10 text-primary border border-primary/20 font-medium">
          <span className={`w-1.5 h-1.5 rounded-full ${runningCount > 0 ? 'bg-emerald-400 animate-pulse' : 'bg-muted-foreground/40'}`} />
          {runningCount}/{listeners.length} listening
        </span>
      }
    >
      <SettingsGroup label="Listeners">
        {listeners.map((listener) => {
          if (editingId === listener.id) {
            const { host, port } = splitAddress(listener.address);
            return (
              <div key={listener.id} className="px-3 py-2.5 bg-primary/5 space-y-1.5">
                <span className="text-2xs text-primary font-semibold font-mono flex items-center gap-1.5">
                  <MingCuteIcon name="edit_line" size={12} />
                  Editing #{listener.id}
                </span>
                <ListenerForm
                  initial={{ label: listener.label, host, port }}
                  listeners={listeners}
                  hostIps={hostIps}
                  ignoreId={listener.id}
                  submitLabel="Save"
                  submitIcon="check_line"
                  onCancel={() => setEditingId(null)}
                  onSubmit={async (label, address, replace) => {
                    await run(
                      () => updateListener(listener.id, label, address, replace),
                      `Listener #${listener.id} updated`
                    );
                    setEditingId(null);
                  }}
                />
              </div>
            );
          }

          const status = statusOf(listener);
          return (
            <div
              key={listener.id}
              className={`px-3 py-2 text-xs font-mono transition-opacity ${
                listener.enabled ? '' : 'opacity-60'
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <Switch
                    checked={listener.enabled}
                    title={listener.enabled ? 'Disable listener' : 'Enable listener'}
                    onChange={() =>
                      run(
                        () => setListenerEnabled(listener.id, !listener.enabled),
                        `${listener.label} ${listener.enabled ? 'disabled' : 'enabled'}`
                      ).catch(() => {})
                    }
                  />
                  <span className="text-3xs text-muted-foreground shrink-0">#{listener.id}</span>
                  <span className="font-semibold text-foreground truncate">{listener.label}</span>
                  <span className="text-muted-foreground text-2xs truncate">{listener.address}</span>
                </div>

                <div className="flex items-center gap-1 shrink-0">
                  <span className="flex items-center gap-1 text-3xs text-muted-foreground mr-1" title={listener.error ?? status.text}>
                    <span className={`w-1.5 h-1.5 rounded-full ${status.dot}`} />
                    {status.text}
                  </span>
                  <button
                    type="button"
                    onClick={() => {
                      setEditingId(listener.id);
                      setFeedback(null);
                    }}
                    className="text-muted-foreground hover:text-foreground p-1 rounded hover:bg-neutral-subtle transition-colors cursor-pointer"
                    title="Edit"
                  >
                    <MingCuteIcon name="edit_line" size={13} />
                  </button>
                  {listener.id === 0 ? (
                    <span className="text-muted-foreground/30 p-1 cursor-not-allowed" title="Listener #0 cannot be removed; disable it instead">
                      <MingCuteIcon name="lock_line" size={13} />
                    </span>
                  ) : (
                    <button
                      type="button"
                      onClick={() =>
                        run(() => removeListener(listener.id), `${listener.label} removed`).catch(() => {})
                      }
                      className="text-muted-foreground hover:text-rose-400 p-1 rounded hover:bg-neutral-subtle transition-colors cursor-pointer"
                      title="Remove"
                    >
                      <MingCuteIcon name="delete_2_line" size={13} />
                    </button>
                  )}
                </div>
              </div>
              {listener.enabled && listener.error && (
                <div className="mt-1 pl-9 text-3xs text-rose-400 break-all">{listener.error}</div>
              )}
            </div>
          );
        })}
      </SettingsGroup>

      <SettingsGroup label="Add listener">
        <div className="px-3 py-2.5">
          <ListenerForm
            listeners={listeners}
            hostIps={hostIps}
            hostPreset={hostPreset}
            submitLabel="Add"
            submitIcon="add_line"
            onSubmit={(label, address, replace) =>
              run(() => addListener(label, address, replace), `Listener '${label}' added on ${address}`)
            }
          />
        </div>
      </SettingsGroup>

      {feedback && (
        <div
          className={`flex items-center gap-2 p-2 rounded-lg border font-mono text-2xs animate-fade-in ${
            feedback.type === 'success'
              ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
              : 'bg-rose-500/10 border-rose-500/30 text-rose-400'
          }`}
        >
          <MingCuteIcon name={feedback.type === 'success' ? 'check_line' : 'alert_line'} size={13} className="shrink-0" />
          <span className="break-all">{feedback.message}</span>
        </div>
      )}

      <SettingsGroup label="Host addresses">
        <div className="px-3 py-2 flex items-center gap-2 text-2xs text-muted-foreground">
          <span className="flex-1">
            IPs on this machine. Point a device at one of these, or bind a listener to it to tell clients apart.
          </span>
          {hiddenIpCount > 0 && (
            <button
              onClick={() => setShowAllIps(!showAllIps)}
              className="text-muted-foreground hover:text-foreground cursor-pointer shrink-0"
            >
              {showAllIps ? 'Hide IPv6 & link-local' : `Show IPv6 & link-local (${hiddenIpCount})`}
            </button>
          )}
          <button
            onClick={loadHostIps}
            className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer shrink-0"
            title="Re-read network interfaces"
          >
            <MingCuteIcon name="refresh_line" size={13} />
          </button>
        </div>

        {hostIpsError && <div className="px-3 py-2 text-rose-500 font-mono text-2xs break-all">{hostIpsError}</div>}
        {!hostIpsError && shownIps.length === 0 && (
          <div className="px-3 py-3 text-muted-foreground italic text-2xs">No addresses found</div>
        )}

        {shownIps.map((h) => {
          const boundBy = listenersOn(h.ip);
          // An all-interfaces listener accepts connections to this IP too (:: also takes IPv4 on dual-stack hosts)
          const coveredBy = wildcardListeners.filter(
            (l) => l.enabled && (!h.isIpv6 || normalizeHost(splitAddress(l.address).host) === '::')
          );
          return (
            <div key={`${h.interface}-${h.ip}`} className="group px-3 py-1.5 flex items-center gap-2 font-mono text-xs">
              <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${h.isUp ? 'bg-emerald-400' : 'bg-muted-foreground/40'}`} title={h.isUp ? 'Interface up' : 'Interface down'} />
              <span className="text-foreground select-text">{h.ip}</span>
              <span className="text-3xs text-muted-foreground">{h.interface}</span>
              {h.isLoopback && <span className="text-3xs px-1 rounded bg-neutral-subtle text-muted-foreground">this machine only</span>}
              {h.isLinkLocal && <span className="text-3xs px-1 rounded bg-neutral-subtle text-muted-foreground">link-local</span>}

              <span className="ml-auto flex items-center gap-1.5 min-w-0">
                {boundBy.map((l) => (
                  <span key={l.id} className="text-3xs px-1.5 py-px rounded bg-primary/10 text-primary truncate" title={l.address}>
                    {l.label} :{splitAddress(l.address).port}
                  </span>
                ))}
                {boundBy.length === 0 && coveredBy.length > 0 && (
                  <span
                    className="text-3xs text-muted-foreground truncate"
                    title={coveredBy.map((l) => `${l.label} (${l.address})`).join(', ')}
                  >
                    via all interfaces :{coveredBy.map((l) => splitAddress(l.address).port).join(', :')}
                  </span>
                )}
                <button
                  onClick={() => copyIp(h.ip)}
                  className="opacity-0 group-hover:opacity-100 p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer transition-opacity"
                  title="Copy IP"
                >
                  <MingCuteIcon name={copiedIp === h.ip ? 'check_line' : 'copy_line'} size={12} />
                </button>
                <button
                  onClick={() => setHostPreset({ host: h.ip, nonce: Date.now() })}
                  className="opacity-0 group-hover:opacity-100 px-1.5 py-0.5 rounded text-2xs font-sans text-muted-foreground hover:text-foreground hover:bg-neutral-subtle cursor-pointer transition-opacity"
                  title="Put this IP in the Add listener form"
                >
                  Use
                </button>
              </span>
            </div>
          );
        })}
      </SettingsGroup>
    </SettingsSection>
  );
};
