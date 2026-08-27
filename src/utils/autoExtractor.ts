import type { ExtractRuleItem, HeaderItem } from '../services/tauri/bridge';

export interface ExtractionResult {
  variableName: string;
  value: string;
}

export function evaluateExtractRule(
  rule: ExtractRuleItem,
  responseBody: string,
  responseHeaders: HeaderItem[] = []
): string | null {
  if (!rule.enabled || !rule.targetVariable || !rule.expression) return null;

  const mode = rule.type || 'json';

  try {
    if (mode === 'json') {
      const parsed = JSON.parse(responseBody);
      const parts = rule.expression.trim().split('.');
      let current: any = parsed;
      for (const part of parts) {
        if (current === null || current === undefined) return null;
        current = current[part];
      }
      return current !== null && current !== undefined ? String(current) : null;
    }

    if (mode === 'header') {
      const targetKey = rule.expression.trim().toLowerCase();
      const match = responseHeaders.find((h) => h.key.trim().toLowerCase() === targetKey);
      return match ? match.value : null;
    }

    if (mode === 'after_string') {
      const [prefix, limitStr] = rule.expression.split('||');
      if (!prefix) return null;
      const limit = parseInt(limitStr || '256', 10);
      const idx = responseBody.indexOf(prefix);
      if (idx === -1) return null;

      const startIdx = idx + prefix.length;
      let substring = responseBody.slice(startIdx, startIdx + limit);
      // Stop at newline if present
      const newlineIdx = substring.search(/[\r\n]/);
      if (newlineIdx !== -1) {
        substring = substring.slice(0, newlineIdx);
      }
      return substring;
    }

    if (mode === 'before_string') {
      const [suffix, limitStr] = rule.expression.split('||');
      if (!suffix) return null;
      const limit = parseInt(limitStr || '256', 10);
      const idx = responseBody.indexOf(suffix);
      if (idx === -1) return null;

      const startIdx = Math.max(0, idx - limit);
      let substring = responseBody.slice(startIdx, idx);
      const lines = substring.split(/[\r\n]/);
      return lines[lines.length - 1]; // Return last line before suffix
    }

    if (mode === 'between_string') {
      const [startDelim, endDelim] = rule.expression.split('||');
      if (!startDelim || !endDelim) return null;

      const startIdx = responseBody.indexOf(startDelim);
      if (startIdx === -1) return null;

      const contentStart = startIdx + startDelim.length;
      const endIdx = responseBody.indexOf(endDelim, contentStart);
      if (endIdx === -1) return null;

      return responseBody.slice(contentStart, endIdx);
    }

    if (mode === 'body_regex' || mode === 'regex') {
      const regex = new RegExp(rule.expression);
      const match = regex.exec(responseBody);
      if (!match) return null;
      return match[1] ?? match[0];
    }
  } catch (err) {
    console.error('Failed to evaluate extract rule:', rule, err);
  }

  return null;
}

export function processAutoExtractionRules(
  rules: ExtractRuleItem[],
  responseBody: string,
  responseHeaders: HeaderItem[] = []
): ExtractionResult[] {
  const results: ExtractionResult[] = [];

  for (const rule of rules) {
    const extracted = evaluateExtractRule(rule, responseBody, responseHeaders);
    if (extracted !== null && extracted.trim().length > 0) {
      results.push({
        variableName: rule.targetVariable.trim().toUpperCase(),
        value: extracted,
      });
    }
  }

  return results;
}
