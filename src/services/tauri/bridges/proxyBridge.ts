import { invoke } from "@tauri-apps/api/core";

export interface ProxyConfig {
  proxyEnabled: boolean;
  interceptEnabled: boolean;
  interceptMode: "request" | "response" | "both";
  proxyMode: "on" | "off" | "block_client" | "block";
  /** Which listeners' traffic may be intercepted */
  interceptSourceScope?: SourceScope;
}

export const getProxyState = async (): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("get_proxy_state");
};

export const setProxyMode = async (
  mode: "on" | "off" | "block_client" | "block"
): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("set_proxy_mode", { mode });
};

export const toggleProxy = async (enabled: boolean): Promise<ProxyConfig> => {
  return await invoke<ProxyConfig>("toggle_proxy", { enabled });
};

import type { ListenerConfig, SourceScope } from "../../../types";

export const getListenerConfigs = async (): Promise<ListenerConfig[]> => {
  return await invoke<ListenerConfig[]>("get_listener_configs");
};

export interface HostIp {
  /** OS interface name, e.g. eth0, wlan0, lo */
  interface: string;
  ip: string;
  isIpv6: boolean;
  isLoopback: boolean;
  isLinkLocal: boolean;
  isUp: boolean;
}

/** IP addresses on this machine's interfaces, usable LAN IPv4 first */
export const listHostIps = async (): Promise<HostIp[]> => {
  return await invoke<HostIp[]>("list_host_ips");
};

export const addListener = async (
  label: string,
  address: string,
  replaceConflicts = false
): Promise<ListenerConfig[]> => {
  return await invoke<ListenerConfig[]>("add_listener", { label, address, replaceConflicts });
};

export const removeListener = async (
  listenerId: number
): Promise<ListenerConfig[]> => {
  return await invoke<ListenerConfig[]>("remove_listener", { listenerId });
};

export const updateListener = async (
  listenerId: number,
  label?: string,
  address?: string,
  replaceConflicts = false
): Promise<ListenerConfig[]> => {
  return await invoke<ListenerConfig[]>("update_listener", {
    listenerId,
    label: label ?? null,
    address: address ?? null,
    replaceConflicts,
  });
};

export const setListenerEnabled = async (
  listenerId: number,
  enabled: boolean
): Promise<ListenerConfig[]> => {
  return await invoke<ListenerConfig[]>("set_listener_enabled", { listenerId, enabled });
};
