import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, Event, UnlistenFn, EventCallback } from '@tauri-apps/api/event';

// This is a global variable to hold the logger function
// It will be set by the useTrafficState hook
let logger: ((entry: any) => void) | null = null;
let isDebugEnabled = false;

export const setDebugLogger = (newLogger: (entry: any) => void, enabled: boolean) => {
  logger = newLogger;
  isDebugEnabled = enabled;
};

export async function invoke<T>(command: string, args?: any): Promise<T> {
  if (isDebugEnabled && logger) {
    logger({
      type: 'invoke',
      target: command,
      payload: args,
      direction: 'to-backend'
    });
  }

  try {
    const result = await tauriInvoke<T>(command, args);
    
    if (isDebugEnabled && logger) {
      logger({
        type: 'response',
        target: command,
        payload: result,
        direction: 'from-backend'
      });
    }
    
    return result;
  } catch (error) {
    if (isDebugEnabled && logger) {
      logger({
        type: 'error',
        target: command,
        payload: error,
        direction: 'from-backend'
      });
    }
    throw error;
  }
}

export async function listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
  const wrappedHandler = (ev: Event<T>) => {
    if (isDebugEnabled && logger) {
      logger({
        type: 'event',
        target: event,
        payload: ev.payload,
        direction: 'from-backend'
      });
    }
    return handler(ev);
  };

  return tauriListen(event, wrappedHandler);
}
