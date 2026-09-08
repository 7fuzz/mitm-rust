import React, { useState, useEffect, useMemo } from 'react';
import type { TrafficItem } from '../../../types';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Dialog, Button, Checkbox, SegmentedControl } from '../../common/ui';
import {
  formatCustomCopy,
  getHttpStatusText,
  DEFAULT_COPY_CUSTOM_OPTIONS,
  type CustomCopyOptions,
} from '../../../utils/reqResFormatter';

interface CopyCustomModalProps {
  isOpen: boolean;
  onClose: () => void;
  item: TrafficItem | null;
}

const STORAGE_KEY = 'mitm_custom_copy_settings';

export const CopyCustomModal: React.FC<CopyCustomModalProps> = ({
  isOpen,
  onClose,
  item,
}) => {
  const [options, setOptions] = useState<CustomCopyOptions>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        return { ...DEFAULT_COPY_CUSTOM_OPTIONS, ...JSON.parse(saved) };
      }
    } catch {
      // ignore
    }
    return DEFAULT_COPY_CUSTOM_OPTIONS;
  });

  const [copied, setCopied] = useState(false);

  // Save options to localStorage whenever they change
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
    } catch {
      // ignore
    }
  }, [options]);

  const previewText = useMemo(() => {
    if (!item) return '';
    return formatCustomCopy(item, options);
  }, [item, options]);

  const handleCopy = async () => {
    if (!previewText) return;
    try {
      await navigator.clipboard.writeText(previewText);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy to clipboard', err);
    }
  };

  const applyPreset = (presetOptions: Partial<CustomCopyOptions>) => {
    setOptions((prev) => ({ ...prev, ...presetOptions }));
  };

  if (!item) return null;

  const lineCount = previewText ? previewText.split('\n').length : 0;
  const charCount = previewText.length;
  const statusCode = item.statusCode || 200;
  const statusText = getHttpStatusText(statusCode);

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Custom Copy Options"
      description="Customize request and response format before copying to clipboard."
      size="lg"
      footer={
        <div className="flex items-center justify-between w-full">
          <div className="text-[11px] text-muted-foreground font-mono">
            {lineCount} lines · {charCount} chars
          </div>
          <div className="flex items-center gap-2">
            <Button variant="subtle" onClick={onClose}>
              Close
            </Button>
            <Button
              variant="primary"
              onClick={handleCopy}
              className="flex items-center gap-1.5"
            >
              <MingCuteIcon
                name={copied ? 'check_line' : 'copy_line'}
                size={14}
                className={copied ? 'text-emerald-400' : ''}
              />
              <span>{copied ? 'Copied to Clipboard!' : 'Copy to Clipboard'}</span>
            </Button>
          </div>
        </div>
      }
    >
      <div className="space-y-4 text-xs font-sans">
        {/* Quick Presets */}
        <div className="flex flex-wrap items-center gap-1.5 pb-2 border-b border-border">
          <span className="text-[10px] uppercase font-bold text-muted-foreground mr-1">
            Presets:
          </span>
          <button
            type="button"
            onClick={() =>
              applyPreset({
                requestFormat: 'curl',
                includeRequestHeaders: true,
                includeRequestBody: true,
                includeResponse: false,
              })
            }
            className="px-2 py-0.5 rounded text-[11px] bg-surface hover:bg-neutral-subtle border border-border text-foreground transition-colors cursor-pointer"
          >
            cURL Only
          </button>
          <button
            type="button"
            onClick={() =>
              applyPreset({
                requestFormat: 'curl',
                includeRequestHeaders: true,
                includeRequestBody: true,
                includeResponse: true,
                includeResponseCode: true,
                includeResponseStatusText: false,
                includeResponseHeaders: false,
                includeResponseBody: true,
              })
            }
            className="px-2 py-0.5 rounded text-[11px] bg-surface hover:bg-neutral-subtle border border-border text-foreground transition-colors cursor-pointer"
          >
            cURL + Response
          </button>
          <button
            type="button"
            onClick={() =>
              applyPreset({
                requestFormat: 'request',
                includeRequestHeaders: false,
                includeRequestBody: true,
                includeResponse: true,
                includeResponseCode: true,
                includeResponseStatusText: false,
                includeResponseHeaders: false,
                includeResponseBody: true,
              })
            }
            className="px-2 py-0.5 rounded text-[11px] bg-surface hover:bg-neutral-subtle border border-border text-foreground transition-colors cursor-pointer"
          >
            URL, Body & Response
          </button>
          <button
            type="button"
            onClick={() =>
              applyPreset({
                requestFormat: 'request',
                includeRequestHeaders: true,
                includeRequestBody: true,
                includeResponse: true,
                includeResponseCode: true,
                includeResponseStatusText: true,
                includeResponseHeaders: true,
                includeResponseBody: true,
              })
            }
            className="px-2 py-0.5 rounded text-[11px] bg-surface hover:bg-neutral-subtle border border-border text-foreground transition-colors cursor-pointer"
          >
            Full HTTP Request & Response
          </button>
        </div>

        {/* Configuration Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Left Column: Request Configuration */}
          <div className="bg-background/50 border border-border rounded-lg p-3 space-y-3">
            <div className="font-semibold text-foreground flex items-center gap-1.5">
              <MingCuteIcon name="send_plane_line" size={14} className="text-primary" />
              <span>Request Format</span>
            </div>

            <div className="space-y-2">
              <label className="block text-[11px] text-muted-foreground font-medium">
                Request Shape:
              </label>
              <SegmentedControl
                options={[
                  { value: 'curl', label: 'cURL Command' },
                  { value: 'request', label: 'Request (HTTP)' },
                ]}
                value={options.requestFormat}
                onChange={(val) =>
                  setOptions((prev) => ({ ...prev, requestFormat: val as 'curl' | 'request' }))
                }
              />
            </div>

            <div className="space-y-2 pt-1 border-t border-border/50">
              <Checkbox
                label="Include Request Headers"
                checked={options.includeRequestHeaders}
                onChange={(e) =>
                  setOptions((prev) => ({ ...prev, includeRequestHeaders: e.target.checked }))
                }
              />
              <div className="block">
                <Checkbox
                  label="Include Request Body"
                  checked={options.includeRequestBody}
                  onChange={(e) =>
                    setOptions((prev) => ({ ...prev, includeRequestBody: e.target.checked }))
                  }
                />
              </div>
              <div className="block">
                <Checkbox
                  label="Pretty-print JSON Bodies"
                  checked={options.prettyJson !== false}
                  onChange={(e) =>
                    setOptions((prev) => ({ ...prev, prettyJson: e.target.checked }))
                  }
                />
              </div>
            </div>
          </div>

          {/* Right Column: Response Configuration */}
          <div className="bg-background/50 border border-border rounded-lg p-3 space-y-3">
            <div className="font-semibold text-foreground flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <MingCuteIcon name="repeat_line" size={14} className="text-emerald-500" />
                <span>Response Format</span>
              </div>
              <Checkbox
                label="Include Response"
                checked={options.includeResponse}
                onChange={(e) =>
                  setOptions((prev) => ({ ...prev, includeResponse: e.target.checked }))
                }
              />
            </div>

            {options.includeResponse ? (
              <div className="space-y-2 pl-2 border-l-2 border-primary/40">
                <div className="block">
                  <Checkbox
                    label={`Response Code (${statusCode})`}
                    checked={options.includeResponseCode}
                    onChange={(e) =>
                      setOptions((prev) => ({ ...prev, includeResponseCode: e.target.checked }))
                    }
                  />
                </div>
                <div className="block">
                  <Checkbox
                    label={`Status Text / Message (${statusText})`}
                    checked={options.includeResponseStatusText}
                    onChange={(e) =>
                      setOptions((prev) => ({
                        ...prev,
                        includeResponseStatusText: e.target.checked,
                      }))
                    }
                  />
                </div>
                <div className="block">
                  <Checkbox
                    label="Include Response Headers"
                    checked={options.includeResponseHeaders}
                    onChange={(e) =>
                      setOptions((prev) => ({
                        ...prev,
                        includeResponseHeaders: e.target.checked,
                      }))
                    }
                  />
                </div>
                <div className="block">
                  <Checkbox
                    label="Include Response Body"
                    checked={options.includeResponseBody}
                    onChange={(e) =>
                      setOptions((prev) => ({
                        ...prev,
                        includeResponseBody: e.target.checked,
                      }))
                    }
                  />
                </div>
              </div>
            ) : (
              <p className="text-[11px] text-muted-foreground italic py-2">
                Response output is omitted.
              </p>
            )}
          </div>
        </div>

        {/* Live Preview */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <label className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
              Live Preview
            </label>
            <button
              type="button"
              onClick={handleCopy}
              className="text-[11px] text-primary hover:underline flex items-center gap-1 cursor-pointer"
            >
              <MingCuteIcon name={copied ? 'check_line' : 'copy_line'} size={12} />
              <span>{copied ? 'Copied!' : 'Quick Copy'}</span>
            </button>
          </div>
          <div className="relative">
            <pre className="w-full bg-background border border-border rounded-lg p-3 text-[11px] font-mono text-foreground overflow-auto max-h-[200px] leading-relaxed whitespace-pre-wrap break-all select-text">
              {previewText || <span className="text-muted-foreground italic">(Nothing selected)</span>}
            </pre>
          </div>
        </div>
      </div>
    </Dialog>
  );
};
