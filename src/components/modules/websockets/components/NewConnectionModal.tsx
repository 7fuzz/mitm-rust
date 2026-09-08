import React, { useState } from 'react';
import { useWebSocketStore } from '../../../../stores/useWebSocketStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button, Input, Dialog } from '../../../common/ui';

interface NewConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const NewConnectionModal: React.FC<NewConnectionModalProps> = ({
  isOpen,
  onClose,
}) => {
  const { createClientConnection, isConnecting } = useWebSocketStore();
  const [connectUrl, setConnectUrl] = useState('wss://echo.websocket.events');
  const [customHeaders, setCustomHeaders] = useState<Array<{ key: string; value: string }>>([
    { key: '', value: '' },
  ]);

  const handleConnectSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!connectUrl.trim()) return;

    const validHeaders = customHeaders
      .filter((h) => h.key.trim() && h.value.trim())
      .map((h) => [h.key.trim(), h.value.trim()] as [string, string]);

    const conn = await createClientConnection(
      connectUrl.trim(),
      validHeaders.length > 0 ? validHeaders : undefined,
    );

    if (conn) {
      onClose();
      setConnectUrl('wss://echo.websocket.events');
      setCustomHeaders([{ key: '', value: '' }]);
    }
  };

  const addHeaderRow = () => {
    setCustomHeaders([...customHeaders, { key: '', value: '' }]);
  };

  const removeHeaderRow = (idx: number) => {
    setCustomHeaders(customHeaders.filter((_, i) => i !== idx));
  };

  const updateHeaderRow = (idx: number, field: 'key' | 'value', val: string) => {
    const updated = [...customHeaders];
    updated[idx][field] = val;
    setCustomHeaders(updated);
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Connect WebSocket Client"
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={handleConnectSubmit}
            disabled={isConnecting || !connectUrl.trim()}
          >
            {isConnecting ? 'Connecting...' : 'Connect to Server'}
          </Button>
        </div>
      }
    >
      <form onSubmit={handleConnectSubmit} className="space-y-4 font-sans text-xs">
        <div>
          <label className="block text-foreground font-medium mb-1">WebSocket URL (ws:// or wss://)</label>
          <Input
            value={connectUrl}
            onChange={(e) => setConnectUrl(e.target.value)}
            placeholder="wss://echo.websocket.events"
            className="font-mono"
            required
          />
          <p className="text-[11px] text-muted-foreground mt-1">
            Connects directly to the specified endpoint with live frame bidirectional streaming.
          </p>
        </div>

        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="block text-foreground font-medium">Custom Handshake Headers (Optional)</label>
            <button
              type="button"
              onClick={addHeaderRow}
              className="text-primary hover:underline font-bold text-[11px]"
            >
              + Add Header
            </button>
          </div>

          <div className="space-y-2 max-h-36 overflow-y-auto">
            {customHeaders.map((header, idx) => (
              <div key={idx} className="flex items-center gap-2">
                <Input
                  placeholder="Header Name (e.g. Authorization)"
                  value={header.key}
                  onChange={(e) => updateHeaderRow(idx, 'key', e.target.value)}
                  className="flex-1 font-mono text-xs"
                />
                <Input
                  placeholder="Header Value"
                  value={header.value}
                  onChange={(e) => updateHeaderRow(idx, 'value', e.target.value)}
                  className="flex-1 font-mono text-xs"
                />
                {customHeaders.length > 1 && (
                  <button
                    type="button"
                    onClick={() => removeHeaderRow(idx)}
                    className="text-muted-foreground hover:text-rose-500 p-1"
                  >
                    <MingCuteIcon name="delete_2_line" size={14} />
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      </form>
    </Dialog>
  );
};
