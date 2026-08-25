import React, { useState } from 'react';
import type { RepeaterHistoryItem } from '../../../types';
import { StatusBadge } from '../../common/StatusBadge';
import { CodeEditor } from '../../common/CodeEditor';
import { KeyValueEditor } from '../../common/KeyValueEditor';
import { MingCuteIcon } from '../../common/MingCuteIcon';

interface ResponsePanelProps {
  response: RepeaterHistoryItem | null;
}

export const ResponsePanel: React.FC<ResponsePanelProps> = ({ response }) => {
  const [activeTab, setActiveTab] = useState<'body' | 'headers'>('body');

  if (!response) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-surface text-muted-foreground p-6 text-xs italic">
        <MingCuteIcon name="send_plane_line" size={36} className="mb-2 opacity-30" />
        Click "Send" above to execute the request and view response headers & payload here.
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col bg-surface overflow-hidden text-xs">
      {/* Response Status Bar */}
      <div className="p-3 bg-header border-b border-border flex items-center justify-between font-mono shrink-0">
        <div className="flex items-center gap-3">
          <StatusBadge code={response.statusCode} />
          <span className="text-muted-foreground">Time: <strong className="text-foreground">{response.durationMs}ms</strong></span>
          <span className="text-muted-foreground">Size: <strong className="text-foreground">{response.size} B</strong></span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={() => setActiveTab('body')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium uppercase ${
              activeTab === 'body' ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Body
          </button>
          <button
            onClick={() => setActiveTab('headers')}
            className={`px-2.5 py-0.5 rounded text-xs font-medium uppercase ${
              activeTab === 'headers' ? 'bg-primary text-primary-foreground font-bold' : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Headers ({response.responseHeaders.length})
          </button>
        </div>
      </div>

      {/* Response Panel Body */}
      <div className="flex-1 p-3 overflow-hidden">
        {activeTab === 'body' && (
          <div className="h-full flex flex-col overflow-hidden">
            <CodeEditor value={response.responseBody} language="json" readOnly />
          </div>
        )}

        {activeTab === 'headers' && (
          <KeyValueEditor
            items={response.responseHeaders.map((h, i) => ({ id: `resh-${i}`, key: h.key, value: h.value, enabled: true }))}
            onChange={() => {}}
            readOnly
          />
        )}
      </div>
    </div>
  );
};
