import { getCurrentWebview } from '@tauri-apps/api/webview';
import { isTauriAvailable } from '../services/tauri/ipc';
import { ZOOM_LEVELS } from '../stores/uiPrefs/registry';

/**
 * Scales the whole UI like browser zoom. In the app this is native webview zoom, which keeps
 * pointer math and Monaco correct; the browser preview falls back to CSS zoom.
 */
export const applyUiZoom = (factor: number) => {
  if (isTauriAvailable()) {
    getCurrentWebview()
      .setZoom(factor)
      .catch((err) => console.error('Failed to set UI zoom:', err));
  } else {
    document.documentElement.style.zoom = String(factor);
  }
};

/** Next zoom level up (+1) or down (-1), staying at the ends */
export const stepZoom = (current: number, direction: 1 | -1): number => {
  const index = ZOOM_LEVELS.findIndex((z) => z === current);
  const next = (index === -1 ? ZOOM_LEVELS.indexOf(1) : index) + direction;
  return ZOOM_LEVELS[Math.min(Math.max(next, 0), ZOOM_LEVELS.length - 1)];
};
