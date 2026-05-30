import { createContext, useContext, useState, ReactNode } from 'react';

type NotificationType = 'success' | 'error' | 'info';

interface Notification {
  id: string;
  message: string;
  type: NotificationType;
}

interface NotificationContextType {
  notify: {
    success: (msg: string) => void;
    error: (msg: string) => void;
    info: (msg: string) => void;
  };
}

const NotificationContext = createContext<NotificationContextType | null>(null);

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notifications, setNotifications] = useState<Notification[]>([]);

  const removeNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  const clearAll = () => {
    setNotifications([]);
  };

  const addNotification = (message: string, type: NotificationType) => {
    const id = Math.random().toString(36).substring(2, 9);
    setNotifications((prev) => [...prev, { id, message, type }]);

    // Auto-dismiss after 3.5 seconds
    setTimeout(() => {
      removeNotification(id);
    }, 3500);
  };

  const notify = {
    success: (msg: string) => addNotification(msg, 'success'),
    error: (msg: string) => addNotification(msg, 'error'),
    info: (msg: string) => addNotification(msg, 'info'),
  };

  return (
    <NotificationContext.Provider value={{ notify }}>
      {children}

      {/* Toast Container */}
      <div
        style={{ position: 'fixed', top: '1.5rem', right: '1.5rem', zIndex: 9999, pointerEvents: 'none' }}
        className="flex flex-col items-end gap-3 w-[350px]"
      >

        {/* Clear All Button */}
        {notifications.length > 1 && (
          <button
            type="button"
            onClick={clearAll}
            style={{ pointerEvents: 'auto' }}
            className="px-3 py-1.5 bg-zinc-900 border border-zinc-800 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 text-[9px] uppercase tracking-widest font-black rounded transition-all shadow-lg shadow-app-shadow/50 animate-in fade-in duration-200 cursor-pointer"
          >
            Clear All
          </button>
        )}

        {notifications.map((n) => (
          <div
            key={n.id}
            className={`pointer-events-auto relative flex items-start px-4 py-3.5 w-full rounded border bg-zinc-950 shadow-2xl shadow-app-shadow animate-in slide-in-from-right-8 fade-in duration-200
              ${n.type === 'success' ? 'border-emerald-500/50 text-emerald-text' : ''}
              ${n.type === 'error' ? 'border-rose-500/50 text-rose-400' : ''}
              ${n.type === 'info' ? 'border-sky-500/50 text-sky-text' : ''}
            `}
          >
            <div className="flex items-start gap-3 w-full">
              <span className="shrink-0 flex items-center justify-center mt-0.5">
                {n.type === 'success' && <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>}
                {n.type === 'error' && <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>}
                {n.type === 'info' && <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>}
              </span>

              <span className="text-[11px] font-mono uppercase tracking-widest font-bold leading-relaxed break-words">
                {n.message}
              </span>
            </div>
          </div>
        ))}
      </div>
    </NotificationContext.Provider>
  );
}

export function useNotification() {
  const context = useContext(NotificationContext);
  if (!context) throw new Error('useNotification must be used within a NotificationProvider');
  return context;
}
