import React, { useState } from 'react';
import { useFuzzerStore } from '../../../stores/useFuzzerStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { Select } from '../../common/ui';
import { PaneHeader } from '../../common/PaneHeader';
import { HTTP_METHODS } from '../collections/tab/CollectionAddressBar';
import { countOccurrences, insertMarkerInFocused, parseVariables, stripMarkers } from '../../../utils/fuzzerMarkers';

type Tab = 'params' | 'headers' | 'body';

export const FuzzerRequestEditor: React.FC = () => {
  const { template, setTemplate, variables } = useFuzzerStore();
  const [tab, setTab] = useState<Tab>('body');
  const readOnly = useFuzzerStore((s) => s.phase === 'running');

  const addMarker = () => {
    // insertMarkerInFocused fires an input event the field's onChange handles; if no
    // field is focused, wrap the whole URL as a convenient default.
    if (insertMarkerInFocused(template) === null) {
      const name = parseVariables(template).includes('var1') ? `var${parseVariables(template).length + 1}` : 'var1';
      setTemplate({ url: `${template.url}{{${name}}}` });
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
          title="Wrap the selected text in a {{variable}} placeholder"
        >
          <span>Mark {'{{ }}'}</span>
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
            valuePlaceholder="Value with {{var}}"
            readOnly={readOnly}
          />
        )}
        {tab === 'headers' && (
          <KeyValueEditor
            items={template.headers}
            onChange={(headers) => setTemplate({ headers })}
            keyPlaceholder="Header name"
            valuePlaceholder="Value with {{var}}"
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
          {variables.length === 0 ? (
            <>Type <span className="text-amber-500 font-mono font-semibold">{'{{name}}'}</span> or select text and press <span className="text-amber-500 font-semibold">Mark</span></>
          ) : (
            <>
              <span className="text-foreground font-semibold">{variables.length}</span> variable{variables.length === 1 ? '' : 's'}
            </>
          )}
        </span>
        {variables.map((v) => {
          const uses = countOccurrences(template, v.name);
          return (
            <span key={v.name} className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-500 font-mono">
              <span className="text-foreground">{`{{${v.name}}}`}</span>
              {uses > 1 && <span className="text-3xs text-amber-500/70">×{uses}</span>}
            </span>
          );
        })}
        {variables.length > 0 && (
          <button
            onClick={() =>
              setTemplate({
                url: stripMarkers(template.url),
                body: stripMarkers(template.body ?? ''),
                params: template.params.map((p) => ({ ...p, value: stripMarkers(p.value) })),
                headers: template.headers.map((h) => ({ ...h, value: stripMarkers(h.value) })),
              })
            }
            className="ml-auto flex items-center gap-1 text-muted-foreground hover:text-rose-500 cursor-pointer"
            title="Remove all {{ }} markers"
          >
            <MingCuteIcon name="close_line" size={11} />
            Clear
          </button>
        )}
      </div>
    </div>
  );
};
