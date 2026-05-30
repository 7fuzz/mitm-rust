import { useState, useEffect, useCallback } from 'react';
import { Modal, Button } from '../ui';
import HttpResponseViewer from '../ui/HttpResponseViewer';
import { TrafficItem } from '../Sidebar/TrafficItem';

interface HistoryItem {
  id: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string;
  response: {
    status: number;
    headers: Record<string, string>;
    body: string;
  };
  timestamp: number;
}

interface RepeaterHistoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  repeaterId: string;
  repeaterName: string;
}

export function RepeaterHistoryModal({ isOpen, onClose, repeaterId, repeaterName }: RepeaterHistoryModalProps) {
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [selectedItem, setSelectedItem] = useState<HistoryItem | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const fetchHistory = useCallback(async () => {
    setIsLoading(true);
    try {
      const res = await fetch(`/api/repeater/${repeaterId}/history`);
      const data = await res.json();
      setHistory(data);
      if (data.length > 0) setSelectedItem(data[0]);
    } catch (error) {
      console.error('Failed to fetch history:', error);
    } finally {
      setIsLoading(false);
    }
  }, [repeaterId]);

  useEffect(() => {
    if (isOpen && repeaterId) {
      fetchHistory();
    }
  }, [isOpen, repeaterId, fetchHistory]);

  const clearHistory = async () => {
    if (!confirm('Are you sure you want to clear the history for this request?')) return;
    try {
      await fetch(`/api/repeater/${repeaterId}/history`, { method: 'DELETE' });
      setHistory([]);
      setSelectedItem(null);
    } catch (error) {
      console.error('Failed to clear history:', error);
    }
  };

  const deleteHistoryItem = async (itemId: string) => {
    if (!confirm('Delete this history item?')) return;
    try {
      await fetch(`/api/repeater-history/${itemId}`, { method: 'DELETE' });
      setHistory(prev => prev.filter(item => item.id !== itemId));
      if (selectedItem?.id === itemId) {
        setSelectedItem(history.find(item => item.id !== itemId) || null);
      }
    } catch (error) {
      console.error('Failed to delete history item:', error);
    }
  };

  const buildRawRequest = (item: HistoryItem) => {
    let path = item.url;
    try {
      const parsed = new URL(item.url);
      path = parsed.pathname + parsed.search + parsed.hash;
    } catch { }
    const firstLine = `${item.method} ${path} HTTP/1.1`;
    const headers = Object.entries(item.headers).map(([k, v]) => `${k}: ${v}`).join('\n');
    return `${firstLine}\n${headers}\n\n${item.body}`;
  };

  const buildRawResponse = (item: HistoryItem) => {
    const firstLine = `HTTP/1.1 ${item.response.status}`;
    const headers = Object.entries(item.response.headers).map(([k, v]) => `${k}: ${v}`).join('\n');
    return `${firstLine}\n${headers}\n\n${item.response.body}`;
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Request History: ${repeaterName}`}
      maxWidth="5xl"
      className="h-[85vh]"
      footer={
        <div className="flex items-center justify-between">
          <Button
            variant="destructive"
            size="sm"
            onClick={clearHistory}
            disabled={history.length === 0}
          >
            Clear All History
          </Button>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
      }
    >
      <div className="flex h-full overflow-hidden min-h-[500px]">
        {/* Sidebar - List of history items */}
        <div className="w-72 border-r border-zinc-800 overflow-y-auto bg-zinc-950">
          {isLoading ? (
            <div className="p-10 text-center text-zinc-600 animate-pulse uppercase text-[10px] font-bold tracking-widest">Loading...</div>
          ) : history.length === 0 ? (
            <div className="p-10 text-center text-zinc-600 uppercase text-[10px] font-bold tracking-widest">No history yet</div>
          ) : (
            history.map((item) => (
              <TrafficItem
                key={item.id}
                id={item.id}
                method={item.method}
                status={item.response.status}
                title={item.url}
                timestamp={item.timestamp}
                isActive={selectedItem?.id === item.id}
                activeColor="purple"
                onClick={() => setSelectedItem(item)}
                onDelete={deleteHistoryItem}
              />
            ))
          )}
        </div>

        {/* Main Viewer */}
        <div className="flex-1 overflow-hidden flex flex-col bg-zinc-900/20">
          {selectedItem ? (
            <div className="flex-1 flex flex-col p-6 space-y-6 overflow-y-auto">
              <div className="space-y-3">
                <h3 className="text-purple-500 font-bold uppercase text-[9px] tracking-widest"># Captured_Request</h3>
                <div className="border border-zinc-800 rounded bg-zinc-950 min-h-[200px] shadow-inner shadow-app-shadow/50">
                  <HttpResponseViewer text={buildRawRequest(selectedItem)} />
                </div>
              </div>
              <div className="space-y-3">
                <h3 className="text-amber-500 font-bold uppercase text-[9px] tracking-widest"># Captured_Response</h3>
                <div className="border border-zinc-800 rounded bg-zinc-950 min-h-[200px] shadow-inner shadow-app-shadow/50">
                  <HttpResponseViewer text={buildRawResponse(selectedItem)} />
                </div>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-zinc-700 uppercase font-black tracking-tighter text-2xl opacity-20 select-none">
              Select an Item
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
