import React, { useState } from 'react';
import type { WebSocketMessage } from '../../../../types';
import { CodeEditor } from '../../../common/CodeEditor';
import { HexViewer } from '../../../common/HexViewer';
import { Button } from '../../../common/ui';

interface WebSocketMessageInspectorProps {
  selectedMsg: WebSocketMessage | null;
  heightPercent: number;
}

export const WebSocketMessageInspector: React.FC<WebSocketMessageInspectorProps> = ({
  selectedMsg,
  heightPercent,
}) => {
  const [activeTab, setActiveTab] = useState<'pretty' | 'raw' | 'hex' | 'info'>('pretty');
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  return (
    <div
      style={{ height: `${heightPercent}%` }}
      className="flex flex-col bg-background overflow-hidden min-h-[100px]"
    >
      <div className="p-2 bg-header border-b border-border flex items-center justify-between shrink-0 font-mono">
        <div className="flex items-center gap-1 bg-surface border border-border rounded p-0.5">
          {(['pretty', 'raw', 'hex', 'info'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-2.5 py-0.5 rounded text-xs font-sans font-medium transition-colors ${
                activeTab === tab
                  ? 'bg-primary text-primary-foreground font-semibold'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab === 'pretty'
                ? 'Pretty JSON'
                : tab === 'raw'
                ? 'Raw Text'
                : tab === 'hex'
                ? 'Hex Viewer'
                : 'Frame Info'}
            </button>
          ))}
        </div>

        {selectedMsg && (
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="xs"
              icon={copiedKey === 'payload-copy' ? 'check_line' : 'copy_line'}
              onClick={() => handleCopy(selectedMsg.payload, 'payload-copy')}
            >
              {copiedKey === 'payload-copy' ? 'Copied' : 'Copy Payload'}
            </Button>
          </div>
        )}
      </div>

      <div className="flex-1 p-2 overflow-hidden bg-surface">
        {selectedMsg ? (
          activeTab === 'hex' ? (
            <HexViewer content={selectedMsg.payload} />
          ) : activeTab === 'pretty' ? (
            <CodeEditor
              value={selectedMsg.payload}
              language={
                selectedMsg.payload.startsWith('{') || selectedMsg.payload.startsWith('[')
                  ? 'json'
                  : 'plaintext'
              }
              readOnly
            />
          ) : activeTab === 'raw' ? (
            <CodeEditor value={selectedMsg.payload} language="plaintext" readOnly />
          ) : (
            <div className="p-3 space-y-2 font-mono text-xs">
              <div className="grid grid-cols-2 gap-2 pb-2 border-b border-border">
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Message ID:</span>
                  <span className="text-foreground">{selectedMsg.id}</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Direction:</span>
                  <span className="font-bold text-primary">{selectedMsg.direction}</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Format:</span>
                  <span className="uppercase text-foreground">{selectedMsg.msg_type}</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Length:</span>
                  <span className="text-foreground">{selectedMsg.length || selectedMsg.payload.length} Bytes</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Timestamp:</span>
                  <span className="text-foreground">{new Date(selectedMsg.timestamp).toLocaleString()}</span>
                </div>
                <div>
                  <span className="text-muted-foreground text-[11px] block font-sans">Injected:</span>
                  <span className="text-foreground">{selectedMsg.is_injected ? 'Yes (User Injected)' : 'No (Wire Captured)'}</span>
                </div>
              </div>
            </div>
          )
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground italic font-sans">
            Select a frame row above to inspect its contents
          </div>
        )}
      </div>
    </div>
  );
};
