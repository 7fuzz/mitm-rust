import { create } from 'zustand';
import type { ProxyState } from './proxy/types';
import { createProxyServerSlice } from './proxy/proxyServerSlice';
import { createTrafficSlice, mapHistoryEntryToTrafficItem, mapHeaders } from './proxy/trafficSlice';
import { createInterceptSlice } from './proxy/interceptSlice';

export const useProxyStore = create<ProxyState>()((...a) => ({
  ...createProxyServerSlice(...a),
  ...createTrafficSlice(...a),
  ...createInterceptSlice(...a),
  initStore: async () => {
    await a[1]().initProxyServer();
    await a[1]().initTraffic();
  },
}));

export * from './proxy/types';
export { mapHistoryEntryToTrafficItem, mapHeaders };
