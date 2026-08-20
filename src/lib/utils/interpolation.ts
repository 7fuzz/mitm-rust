import { GlobalVariable } from '@/hooks/traffic/types';

export function interpolateVariables(
  text: string,
  variables: GlobalVariable[] = [],
  activeEnvId?: string
): string {
  if (!text) return text;
  const varMap: Record<string, string> = {};

  // Populate all baseline variables
  variables.forEach((v) => {
    const val = v.values[v.activeIndex]?.value ?? v.values[0]?.value ?? '';
    varMap[v.name] = val;
  });

  // Override with active environment variables if set
  if (activeEnvId) {
    variables
      .filter((v) => v.environmentId === activeEnvId)
      .forEach((v) => {
        const val = v.values[v.activeIndex]?.value ?? v.values[0]?.value ?? '';
        varMap[v.name] = val;
      });
  }

  let result = text;
  Object.entries(varMap).forEach(([k, v]) => {
    if (!k) return;
    result = result.replaceAll(`{{${k}}}`, v);
    result = result.replaceAll(`%7B%7B${k}%7D%7D`, encodeURIComponent(v));
  });

  return result;
}

export function buildCurlCommand(
  method: string,
  url: string,
  headers: [string, string][] | Record<string, string> | any[],
  body: string,
  variables: GlobalVariable[] = [],
  activeEnvId?: string
): string {
  const finalUrl = interpolateVariables(url, variables, activeEnvId);
  let command = `curl -X ${method} "${finalUrl}"`;

  const headerPairs: [string, string][] = Array.isArray(headers)
    ? headers.map((h) => (Array.isArray(h) ? [h[0], h[1]] : [h.key || h.name || '', h.value || '']))
    : Object.entries(headers || {});

  headerPairs.forEach(([k, v]) => {
    const finalK = interpolateVariables(k, variables, activeEnvId);
    const finalV = interpolateVariables(v, variables, activeEnvId);
    if (finalK.trim()) {
      command += ` -H "${finalK}: ${finalV}"`;
    }
  });

  if (body && method !== 'GET' && method !== 'HEAD') {
    const finalBody = interpolateVariables(body, variables, activeEnvId);
    const escapedBody = finalBody.replace(/"/g, '\\"');
    command += ` -d "${escapedBody}"`;
  }

  return command;
}
