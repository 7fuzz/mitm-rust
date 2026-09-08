import React from 'react';
import type { WebhookDelivery } from '../../../../types';

interface WebhookDeliveriesTableProps {
  widthPercent: number;
  deliveries: WebhookDelivery[];
  filteredDeliveries: WebhookDelivery[];
  selectedDeliveryId: string | null;
  onSelectDelivery: (id: string) => void;
}

export const WebhookDeliveriesTable: React.FC<WebhookDeliveriesTableProps> = ({
  widthPercent,
  deliveries,
  filteredDeliveries,
  selectedDeliveryId,
  onSelectDelivery,
}) => {
  return (
    <div
      style={{ width: `${widthPercent}%` }}
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
              const isSelected = selectedDeliveryId === del.id;
              return (
                <tr
                  key={del.id}
                  onClick={() => onSelectDelivery(del.id)}
                  className={`cursor-pointer transition-colors ${
                    isSelected
                      ? 'bg-primary/15 font-semibold text-foreground ring-1 ring-inset ring-primary'
                      : 'hover:bg-neutral-subtle'
                  }`}
                >
                  <td
                    className="px-3 py-2 text-primary font-bold truncate max-w-[140px]"
                    title={del.endpointPath}
                  >
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
  );
};
