import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { WEBHOOK_TEMPLATES, WebhookTemplate } from '@/types/webhook';
import { Modal } from '../ui/Modal';
import { useNotification } from '../ui/NotificationProvider';

interface RepeaterWebhookModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeGroupId: string;
  onSuccess: (newId: string) => void;
}

export function RepeaterWebhookModal({
  isOpen,
  onClose,
  activeGroupId,
  onSuccess,
}: RepeaterWebhookModalProps) {
  const { notify } = useNotification();
  const [selectedWhTmpl, setSelectedWhTmpl] = useState<WebhookTemplate>(WEBHOOK_TEMPLATES[0]);
  const [whSecret, setWhSecret] = useState('whsec_test_secret_12345');
  const [whTargetUrl, setWhTargetUrl] = useState('http://localhost:3000/api/webhooks');
  const [isCreating, setIsCreating] = useState(false);

  if (!isOpen) return null;

  const handleCreate = async () => {
    setIsCreating(true);
    try {
      let headers: [string, string][] = [['Content-Type', 'application/json']];

      if (selectedWhTmpl.defaultHeader) {
        try {
          const sig: { header: string; value: string } = await invoke('calculate_webhook_signature', {
            payload: selectedWhTmpl.sampleBody,
            secret: whSecret,
            provider: selectedWhTmpl.provider,
          });
          headers.push([sig.header, sig.value]);
        } catch (err) {
          console.error('Signature calculation failed:', err);
        }
      }

      const newId = await invoke<string>('create_repeater_item', {
        item: {
          name: `Webhook: ${selectedWhTmpl.name}`,
          method: 'POST',
          url: whTargetUrl,
          headers,
          body: selectedWhTmpl.sampleBody,
          groupId: activeGroupId === 'All' || activeGroupId === 'null' ? null : activeGroupId,
          bodyMode: 'json',
          bodyJson: selectedWhTmpl.sampleBody,
        },
      });

      notify.success('Created new webhook request in Repeater');
      onSuccess(newId);
      onClose();
    } catch (err) {
      notify.error(`Failed to create webhook request: ${err}`);
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="⚡ New Webhook Request"
      maxWidth="lg"
      footer={
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={onClose}
            className="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-xs font-semibold cursor-pointer"
          >
            Cancel
          </button>
          <button
            onClick={handleCreate}
            disabled={isCreating}
            className="px-4 py-1.5 bg-purple-500 hover:bg-purple-400 text-zinc-950 font-bold rounded text-xs cursor-pointer"
          >
            {isCreating ? 'Creating...' : 'Create Webhook Request'}
          </button>
        </div>
      }
    >
      <div className="p-4 flex flex-col gap-4">
        <p className="text-xs text-zinc-400">Construct a pre-signed synthetic webhook template into your Repeater collection.</p>

        <div className="flex flex-col gap-3 text-xs">
          <div>
            <label className="text-zinc-400 block mb-1 font-semibold">Provider Template</label>
            <select
              value={selectedWhTmpl.id}
              onChange={(e) => {
                const tmpl = WEBHOOK_TEMPLATES.find((t) => t.id === e.target.value);
                if (tmpl) setSelectedWhTmpl(tmpl);
              }}
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-zinc-200 focus:outline-none focus:border-purple-500 cursor-pointer"
            >
              {WEBHOOK_TEMPLATES.map((tmpl) => (
                <option key={tmpl.id} value={tmpl.id} className="bg-zinc-900 text-zinc-200 py-1 font-mono">
                  {tmpl.name}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="text-zinc-400 block mb-1 font-semibold">Target Endpoint URL</label>
            <input
              type="text"
              value={whTargetUrl}
              onChange={(e) => setWhTargetUrl(e.target.value)}
              placeholder="http://localhost:3000/api/webhooks"
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-zinc-200 focus:outline-none focus:border-purple-500"
            />
          </div>

          {selectedWhTmpl.defaultHeader && (
            <div>
              <label className="text-zinc-400 block mb-1 font-semibold">
                Signing Secret ({selectedWhTmpl.defaultHeader})
              </label>
              <input
                type="text"
                value={whSecret}
                onChange={(e) => setWhSecret(e.target.value)}
                placeholder="whsec_..."
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-3 py-2 text-purple-300 font-mono focus:outline-none focus:border-purple-500"
              />
            </div>
          )}

          <div>
            <label className="text-zinc-400 block mb-1 font-semibold">Sample Payload</label>
            <pre className="bg-zinc-950 p-3 rounded border border-zinc-800 text-[11px] font-mono text-emerald-400 max-h-40 overflow-y-auto">
              {selectedWhTmpl.sampleBody}
            </pre>
          </div>
        </div>
      </div>
    </Modal>
  );
}
