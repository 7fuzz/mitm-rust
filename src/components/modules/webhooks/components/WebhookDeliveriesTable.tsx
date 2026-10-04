import React from 'react';
import type { WebhookDelivery } from '../../../../types';

interface WebhookDeliveriesTableProps {
  deliveries: WebhookDelivery[];
  selectedDeliveryId: number | null;
  onSelectDelivery: (id: number) => void;
  emptyMessage: string;
}

const SIGNATURE_BADGE: Record<WebhookDelivery['signatureStatus'], string> = {
  valid: 'bg-emerald-500/15 text-emerald-500',
  invalid: 'bg-rose-500/15 text-rose-500',
  none: 'bg-neutral-subtle text-muted-foreground',
};

const formatSize = (bytes: number) => (bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`);

export const WebhookDeliveriesTable: React.FC<WebhookDeliveriesTableProps> = ({
  deliveries,
  selectedDeliveryId,
  onSelectDelivery,
  emptyMessage,
}) => (
  <div className="h-full overflow-y-auto">
    <table className="w-full text-left font-mono text-2xs">
      <thead className="bg-header text-muted-foreground text-3xs uppercase tracking-wider font-sans sticky top-0 z-10">
        <tr className="border-b border-border">
          <th className="px-3 py-1.5 font-semibold">Path</th>
          <th className="w-24 px-2 py-1.5 font-semibold">Time</th>
          <th className="w-16 px-2 py-1.5 font-semibold text-right">Size</th>
          <th className="w-20 px-2 py-1.5 font-semibold text-center">Signature</th>
        </tr>
      </thead>
      <tbody>
        {deliveries.length === 0 ? (
          <tr>
            <td colSpan={4} className="px-3 py-12 text-center text-muted-foreground italic font-sans">
              {emptyMessage}
            </td>
          </tr>
        ) : (
          deliveries.map((d) => (
            <tr
              key={d.id}
              onClick={() => onSelectDelivery(d.id)}
              className={`cursor-pointer border-b border-border/50 ${
                selectedDeliveryId === d.id ? 'bg-primary/10' : 'hover:bg-neutral-subtle/60'
              }`}
            >
              <td
                className={`px-3 py-1.5 truncate max-w-0 border-l-2 ${
                  selectedDeliveryId === d.id ? 'border-l-primary text-foreground font-semibold' : 'border-l-transparent text-foreground'
                }`}
                title={d.endpointPath}
              >
                {d.endpointPath}
              </td>
              <td className="px-2 py-1.5 text-muted-foreground tabular-nums" title={new Date(d.timestamp).toLocaleString()}>
                {new Date(d.timestamp).toLocaleTimeString()}
              </td>
              <td className="px-2 py-1.5 text-muted-foreground text-right tabular-nums">{formatSize(d.payload.length)}</td>
              <td className="px-2 py-1.5 text-center">
                <span className={`px-1.5 py-px rounded text-3xs font-bold uppercase ${SIGNATURE_BADGE[d.signatureStatus]}`}>
                  {d.signatureStatus}
                </span>
              </td>
            </tr>
          ))
        )}
      </tbody>
    </table>
  </div>
);
