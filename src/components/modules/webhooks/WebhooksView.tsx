import React, { useState, useEffect, useRef } from 'react';
import { useWebhookStore } from '../../../stores/useWebhookStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button, Input } from '../../common/ui';
import { WebhookEndpointsSection } from './components/WebhookEndpointsSection';
import { WebhookDeliveriesTable } from './components/WebhookDeliveriesTable';
import { WebhookDeliveryInspector } from './components/WebhookDeliveryInspector';
import { WebhookHmacSandbox } from './components/WebhookHmacSandbox';
import { NewEndpointModal } from './components/NewEndpointModal';

export const WebhooksView: React.FC = () => {
  const {
    listenerConfig,
    deliveries,
    clearDeliveries,
    selectedDeliveryId,
    selectDelivery,
    initialize,
  } = useWebhookStore();

  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Resizable delivery list & inspector
  const [deliveryListPercent, setDeliveryListPercent] = useState<number>(42);
  const deliveryAreaRef = useRef<HTMLDivElement>(null);
  const isResizingDelivery = useRef(false);

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

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-y-auto space-y-4 text-xs font-sans">
      {/* Top Header Banner: Server Controller & Registered Endpoints */}
      <WebhookEndpointsSection onOpenAddModal={() => setIsAddModalOpen(true)} />

      {/* Main Delivery Log & Inspector Area */}
      <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="history_line" size={16} className="text-primary" />
            <span className="font-semibold text-foreground text-sm">
              Delivery Logs & Captured Payloads
            </span>
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

        <div
          ref={deliveryAreaRef}
          className="flex h-[420px] overflow-hidden border border-border rounded"
        >
          {/* Delivery Hits Table */}
          <WebhookDeliveriesTable
            widthPercent={deliveryListPercent}
            deliveries={deliveries}
            filteredDeliveries={filteredDeliveries}
            selectedDeliveryId={selectedDelivery?.id || null}
            onSelectDelivery={selectDelivery}
          />

          {/* Draggable Resizer Handle */}
          <div
            onMouseDown={handleMouseDownDeliverySplit}
            className="w-1.5 bg-border hover:bg-primary/60 active:bg-primary cursor-col-resize flex items-center justify-center transition-colors select-none group z-20 shrink-0"
            title="Drag to resize panels"
          >
            <div className="w-0.5 h-6 rounded bg-muted-foreground/40 group-hover:bg-primary" />
          </div>

          {/* Delivery Inspector & Replayer */}
          <WebhookDeliveryInspector selectedDelivery={selectedDelivery} />
        </div>
      </div>

      {/* Built-in HMAC Signature Sandbox & Generator */}
      <WebhookHmacSandbox />

      {/* Add Webhook Endpoint Modal Dialog */}
      <NewEndpointModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        listenerPort={listenerConfig.port}
      />
    </div>
  );
};
