import React from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import { Button } from './ui/Button';
import type { ExtractRuleItem } from '../../services/tauri/bridge';

export type ExtractionMode =
  | 'json'
  | 'after_string'
  | 'before_string'
  | 'between_string'
  | 'header'
  | 'body_regex';

interface ExtractRulesEditorProps {
  rules: ExtractRuleItem[];
  onChange: (updatedRules: ExtractRuleItem[]) => void;
  availableVariables?: string[];
}

export const ExtractRulesEditor: React.FC<ExtractRulesEditorProps> = ({
  rules,
  onChange,
  availableVariables = [],
}) => {
  const handleRuleChange = (
    index: number,
    field: keyof ExtractRuleItem,
    value: string | boolean
  ) => {
    const updated = rules.map((r, i) => (i === index ? { ...r, [field]: value } : r));
    onChange(updated);
  };

  const handleAddRule = () => {
    const newRule: ExtractRuleItem = {
      id: crypto.randomUUID(),
      type: 'json',
      expression: 'data.token',
      targetVariable: 'AUTH_TOKEN',
      enabled: true,
    };
    onChange([...rules, newRule]);
  };

  const handleDeleteRule = (index: number) => {
    onChange(rules.filter((_, i) => i !== index));
  };

  // Helper parsers and encoders for multi-parameter expressions
  const parseBetweenExpr = (expr: string) => {
    const parts = expr.split('||');
    return { start: parts[0] || '', end: parts[1] || '' };
  };

  const parseBoundedExpr = (expr: string) => {
    const parts = expr.split('||');
    return { str: parts[0] || '', limit: parts[1] || '256' };
  };

  return (
    <div className="space-y-3 font-sans text-xs select-none">
      <div className="flex items-center justify-between text-muted-foreground text-2xs pb-1 border-b border-border">
        <span>
          Define extraction rules (JSON path, String delimiters, Headers, Regex) to auto-populate environment variables.
        </span>
        <Button variant="amber" size="xs" icon="plus_line" onClick={handleAddRule}>
          + Add Rule
        </Button>
      </div>

      {rules.length === 0 ? (
        <div className="py-8 text-center border border-dashed border-border rounded-lg text-muted-foreground italic flex flex-col items-center gap-2">
          <MingCuteIcon name="lightning_line" size={28} className="opacity-30 text-amber-500" />
          <span>No auto-extraction rules configured.</span>
          <Button variant="outline" size="xs" icon="plus_line" onClick={handleAddRule}>
            Create Extraction Rule
          </Button>
        </div>
      ) : (
        <div className="space-y-2 max-h-96 overflow-y-auto pr-1">
          {rules.map((rule, idx) => {
            const mode = (rule.type || 'json') as ExtractionMode;

            return (
              <div
                key={rule.id || idx}
                className="p-2.5 border border-border rounded-lg bg-background flex flex-col gap-2 font-mono"
              >
                {/* Top Row: Enable, Target Variable, Mode Selector, Delete */}
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(e) => handleRuleChange(idx, 'enabled', e.target.checked)}
                    className="cursor-pointer accent-primary shrink-0"
                    title="Enable / Disable Rule"
                  />

                  {/* Target Variable Input */}
                  <div className="flex-1 min-w-[140px]">
                    <input
                      type="text"
                      list={availableVariables.length > 0 ? `vars-list-${idx}` : undefined}
                      value={rule.targetVariable}
                      onChange={(e) => handleRuleChange(idx, 'targetVariable', e.target.value)}
                      placeholder="Target Variable (e.g. AUTH_TOKEN)"
                      className="w-full bg-surface border border-border rounded px-2.5 py-1 text-xs text-primary font-bold focus:outline-none focus:border-primary"
                    />
                    {availableVariables.length > 0 && (
                      <datalist id={`vars-list-${idx}`}>
                        {availableVariables.map((v) => (
                          <option key={v} value={v} />
                        ))}
                      </datalist>
                    )}
                  </div>

                  <span className="text-muted-foreground text-xs font-sans shrink-0">← extract from</span>

                  {/* Mode Selector */}
                  <select
                    value={mode}
                    onChange={(e) => {
                      const newMode = e.target.value as ExtractionMode;
                      let defaultExpr = '';
                      if (newMode === 'json') defaultExpr = 'data.token';
                      else if (newMode === 'after_string') defaultExpr = 'token=||64';
                      else if (newMode === 'before_string') defaultExpr = '&expires=||64';
                      else if (newMode === 'between_string') defaultExpr = 'token="||"';
                      else if (newMode === 'header') defaultExpr = 'Authorization';
                      else if (newMode === 'body_regex') defaultExpr = 'token=([^;]+)';

                      const updated = rules.map((r, i) =>
                        i === idx ? { ...r, type: newMode, expression: defaultExpr } : r
                      );
                      onChange(updated);
                    }}
                    className="bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-semibold focus:outline-none cursor-pointer shrink-0"
                  >
                    <option value="json">JSON Path (json)</option>
                    <option value="after_string">After String (after_string)</option>
                    <option value="before_string">Before String (before_string)</option>
                    <option value="between_string">Between String (between_string)</option>
                    <option value="header">Header Value (header)</option>
                    <option value="body_regex">Regex Pattern (regex)</option>
                  </select>

                  {/* Delete Button */}
                  <button
                    type="button"
                    onClick={() => handleDeleteRule(idx)}
                    className="p-1 text-muted-foreground hover:text-rose-400 rounded transition-colors cursor-pointer shrink-0"
                    title="Delete Rule"
                  >
                    <MingCuteIcon name="delete_2_line" size={15} />
                  </button>
                </div>

                {/* Bottom Row: Mode-Specific Expression Controls */}
                <div className="pl-6 flex items-center gap-2 text-xs">
                  {mode === 'json' && (
                    <input
                      type="text"
                      value={rule.expression}
                      onChange={(e) => handleRuleChange(idx, 'expression', e.target.value)}
                      placeholder="JSON Path (e.g. data.user.token or access_token)"
                      className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                    />
                  )}

                  {mode === 'after_string' && (() => {
                    const { str, limit } = parseBoundedExpr(rule.expression);
                    return (
                      <div className="flex-1 flex items-center gap-2">
                        <input
                          type="text"
                          value={str}
                          onChange={(e) =>
                            handleRuleChange(idx, 'expression', `${e.target.value}||${limit}`)
                          }
                          placeholder="Extract text after string (e.g. token=)"
                          className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                        />
                        <div className="flex items-center gap-1 shrink-0 font-sans text-2xs text-muted-foreground">
                          <span>Max Chars:</span>
                          <input
                            type="number"
                            value={limit}
                            onChange={(e) =>
                              handleRuleChange(idx, 'expression', `${str}||${e.target.value}`)
                            }
                            className="w-16 bg-surface border border-border rounded px-2 py-1 text-xs text-foreground font-mono text-center focus:outline-none focus:border-primary"
                          />
                        </div>
                      </div>
                    );
                  })()}

                  {mode === 'before_string' && (() => {
                    const { str, limit } = parseBoundedExpr(rule.expression);
                    return (
                      <div className="flex-1 flex items-center gap-2">
                        <input
                          type="text"
                          value={str}
                          onChange={(e) =>
                            handleRuleChange(idx, 'expression', `${e.target.value}||${limit}`)
                          }
                          placeholder="Extract text before string (e.g. &expires=)"
                          className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                        />
                        <div className="flex items-center gap-1 shrink-0 font-sans text-2xs text-muted-foreground">
                          <span>Max Chars:</span>
                          <input
                            type="number"
                            value={limit}
                            onChange={(e) =>
                              handleRuleChange(idx, 'expression', `${str}||${e.target.value}`)
                            }
                            className="w-16 bg-surface border border-border rounded px-2 py-1 text-xs text-foreground font-mono text-center focus:outline-none focus:border-primary"
                          />
                        </div>
                      </div>
                    );
                  })()}

                  {mode === 'between_string' && (() => {
                    const { start, end } = parseBetweenExpr(rule.expression);
                    return (
                      <div className="flex-1 flex items-center gap-2">
                        <input
                          type="text"
                          value={start}
                          onChange={(e) =>
                            handleRuleChange(idx, 'expression', `${e.target.value}||${end}`)
                          }
                          placeholder="Start Delimiter (e.g. session=&quot;)"
                          className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                        />
                        <span className="text-muted-foreground shrink-0 font-sans text-2xs">and</span>
                        <input
                          type="text"
                          value={end}
                          onChange={(e) =>
                            handleRuleChange(idx, 'expression', `${start}||${e.target.value}`)
                          }
                          placeholder="End Delimiter (e.g. &quot;)"
                          className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                        />
                      </div>
                    );
                  })()}

                  {mode === 'header' && (
                    <input
                      type="text"
                      value={rule.expression}
                      onChange={(e) => handleRuleChange(idx, 'expression', e.target.value)}
                      placeholder="Response Header Name (e.g. Authorization or Set-Cookie)"
                      className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                    />
                  )}

                  {mode === 'body_regex' && (
                    <input
                      type="text"
                      value={rule.expression}
                      onChange={(e) => handleRuleChange(idx, 'expression', e.target.value)}
                      placeholder="Regex Pattern (e.g. token=([a-zA-Z0-9_-]+))"
                      className="flex-1 bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                    />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
