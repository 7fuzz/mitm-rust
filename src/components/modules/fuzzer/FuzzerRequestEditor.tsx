import React, { useState } from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { Select } from '../../common/ui';
import { PaneHeader } from '../../common/PaneHeader';
import { HTTP_METHODS } from '../collections/tab/CollectionAddressBar';
import { insertMarkerInFocused } from '../../../utils/fuzzerMarkers';

type Tab = 'params' | 'headers' | 'body';

const fieldLabel = (field: string) => field.replace(/^param:/, '').replace(/^header:/, '');

export const FuzzerRequestEditor: React.FC = () => {
  const { template, setTemplate, positions } = useFuzzerStore();
  const [tab, setTab] = useState<Tab>('body');
  const readOnly = useFuzzerStore((s) => s.phase === 'running');

  const addMarker = () => {
    if (!insertMarkerInFocused()) {
      // No field focused: wrap the whole URL as a convenient default
      setTemplate({ url: `§${template.url}§` });
    }
  };

  return (
    <div className="flex flex-col min-h-0 border border-border rounded-lg bg-surface overflow-hidden">
      <div className="h-8 px-2.5 bg-header border-b border-border flex items-center select-none">
        <span className="text-3xs font-semibold uppercase tracking-wider text-muted-foreground">Request</span>
      </div>
      <div className="flex items-center gap-2 p-2 border-b border-border">
        <div className="w-28 shrink-0">
          <Select value={template.method} onChange={(e) => setTemplate({ method: e.target.value })} options={HTTP_METHODS} />
        </div>
        <input
          type="text"
          value={template.url}
          onChange={(e) => setTemplate({ url: e.target.value })}
          placeholder="https://api.example.com/users/1"
          className="flex-1 bg-background border border-border rounded px-2.5 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
        />
        <button
          onMouseDown={(e) => {
            e.preventDefault();
            addMarker();
          }}
          className="flex items-center gap-1 px-2 py-1.5 rounded bg-amber-500/15 border border-amber-500/40 text-amber-500 font-semibold text-xs cursor-pointer hover:bg-amber-500/25 shrink-0"
          title="Wrap the selected text in a §payload§ position"
        >
          <span>Add §</span>
        </button>
      </div>

      <PaneHeader
        tabs={[
          { value: 'params', label: 'Params', count: template.params.length },
          { value: 'headers', label: 'Headers', count: template.headers.length },
          { value: 'body', label: 'Body' },
        ]}
        activeTab={tab}
        onTabChange={(t) => setTab(t as Tab)}
      />

      <div className="flex-1 min-h-0 overflow-auto p-2">
        {tab === 'params' && (
          <KeyValueEditor
            items={template.params}
            onChange={(params) => setTemplate({ params })}
            keyPlaceholder="Param key"
            valuePlaceholder="Value (select, then Add §)"
            readOnly={readOnly}
          />
        )}
        {tab === 'headers' && (
          <KeyValueEditor
            items={template.headers}
            onChange={(headers) => setTemplate({ headers })}
            keyPlaceholder="Header name"
            valuePlaceholder="Value (select, then Add §)"
            readOnly={readOnly}
          />
        )}
        {tab === 'body' && (
          <textarea
            value={template.body ?? ''}
            onChange={(e) => setTemplate({ body: e.target.value })}
            readOnly={readOnly}
            placeholder={'{\n  "role": "user"\n}'}
            className="w-full h-full min-h-[140px] bg-background border border-border rounded p-2 font-mono text-xs text-foreground focus:outline-none focus:border-primary resize-none"
          />
        )}
      </div>

      <div className="px-2 py-1.5 border-t border-border flex items-center gap-2 flex-wrap text-2xs">
        <span className="text-muted-foreground">
          {positions.length === 0 ? (
            <>Select text and press <span className="text-amber-500 font-semibold">Add §</span> to mark a payload position</>
          ) : (
            <>
              <span className="text-foreground font-semibold">{positions.length}</span> position{positions.length === 1 ? '' : 's'}
            </>
          )}
        </span>
        {positions.map((p) => (
          <span key={p.index} className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-500 font-mono">
            <span className="text-3xs text-amber-500/70">{p.index + 1}</span>
            <span className="text-muted-foreground">{fieldLabel(p.field)}</span>
            {p.base && <span className="text-foreground">= {p.base}</span>}
          </span>
        ))}
        {positions.length > 0 && (
          <button
            onClick={() => {
              const strip = (v: string) => v.split('§').join('');
              setTemplate({
                url: strip(template.url),
                body: strip(template.body ?? ''),
                params: template.params.map((p) => ({ ...p, value: strip(p.value) })),
                headers: template.headers.map((h) => ({ ...h, value: strip(h.value) })),
              });
            }}
            className="ml-auto flex items-center gap-1 text-muted-foreground hover:text-rose-500 cursor-pointer"
            title="Remove all § markers"
          >
            <MingCuteIcon name="close_line" size={11} />
            Clear
          </button>
        )}
      </div>
    </div>
  );
};
