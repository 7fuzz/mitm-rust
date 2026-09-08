import React, { useState } from 'react';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { Button, Input, Dialog } from '../../../common/ui';

interface NewEndpointModalProps {
  isOpen: boolean;
  onClose: () => void;
  listenerPort: number;
}

export const NewEndpointModal: React.FC<NewEndpointModalProps> = ({
  isOpen,
  onClose,
  listenerPort,
}) => {
  const { addEndpoint } = useWebhookStore();
  const [newPath, setNewPath] = useState('');
  const [newName, setNewName] = useState('');
  const [newSecret, setNewSecret] = useState('');

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
    onClose();
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Register New Webhook Endpoint"
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
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
            Accessible locally at{' '}
            <code>
              http://localhost:{listenerPort}
              {newPath.startsWith('/') ? newPath : `/${newPath}`}
            </code>
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
  );
};
