import type { RepeaterTab } from '../services/tauri/bridge';
import { buildUrlWithParams } from './urlParams';

export interface CopyAction {
  label: string;
  icon: string;
  iconClass: string;
  text: () => string;
  message: string;
}

/** Copy actions for a Repeater tab as it is in the editor, before variables are filled in. */
export const requestCopyActions = (tab: RepeaterTab): CopyAction[] => {
  const url = () => buildUrlWithParams(tab.url, tab.params || []);
  const headerLines = () =>
    (tab.headers || []).filter((h) => h.enabled && h.key.trim()).map((h) => `${h.key}: ${h.value}`);
  const body = tab.bodyContent || '';

  return [
    { label: 'Copy URL', icon: 'link_line', iconClass: 'text-sky-500', text: url, message: 'Copied URL!' },
    { label: 'Copy Headers', icon: 'list_check_line', iconClass: 'text-emerald-500', text: () => headerLines().join('\n'), message: 'Copied Headers!' },
    { label: 'Copy Body', icon: 'file_text_line', iconClass: 'text-primary', text: () => body, message: 'Copied Body!' },
    {
      label: 'Copy Raw Request',
      icon: 'copy_line',
      iconClass: 'text-amber-500',
      text: () => `${tab.method} ${url()}\n${headerLines().join('\n')}${body ? `\n\n${body}` : ''}`,
      message: 'Copied Full Request!',
    },
    {
      label: 'Copy as cURL',
      icon: 'terminal_line',
      iconClass: 'text-blue-500',
      text: () => {
        const headers = headerLines().map((h) => `-H '${h}'`).join(' ');
        const data = body ? `-d '${body.replace(/'/g, "'\\''")}'` : '';
        return `curl -X ${tab.method} '${url()}' ${headers ? headers + ' ' : ''}${data}`.trim();
      },
      message: 'Copied as cURL!',
    },
  ];
};
