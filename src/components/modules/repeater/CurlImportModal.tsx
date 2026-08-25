import React, { useState } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const CurlImportModal: React.FC = () => {
  const { isCurlModalOpen, setCurlModalOpen, importCurlCommand } = useRepeaterStore();
  const [curlText, setCurlText] = useState('');
  const [reqName, setReqName] = useState('Imported cURL');

  if (!isCurlModalOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!curlText.trim()) return;
    await importCurlCommand(curlText.trim(), reqName.trim());
    setCurlText('');
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
      <div className="w-full max-w-lg bg-surface border border-border rounded-lg shadow-2xl overflow-hidden text-xs">
        <div className="p-3 bg-header border-b border-border font-semibold flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="download_line" size={16} className="text-primary" />
            <span>Import cURL Command</span>
          </div>
          <button onClick={() => setCurlModalOpen(false)} className="text-muted-foreground hover:text-foreground">
            <MingCuteIcon name="close_line" size={14} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-4 space-y-3">
          <div>
            <label className="block text-muted-foreground mb-1 font-medium">Request Name:</label>
            <input
              type="text"
              value={reqName}
              onChange={(e) => setReqName(e.target.value)}
              className="w-full bg-background border border-border rounded px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
            />
          </div>

          <div>
            <label className="block text-muted-foreground mb-1 font-medium">Paste Raw cURL String:</label>
            <textarea
              rows={6}
              value={curlText}
              onChange={(e) => setCurlText(e.target.value)}
              placeholder={`curl -X POST "https://api.example.com/v1/auth" \\\n  -H "Content-Type: application/json" \\\n  -d '{"username": "admin"}'`}
              className="w-full bg-background border border-border rounded p-2.5 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
            />
          </div>

          <div className="flex items-center justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={() => setCurlModalOpen(false)}
              className="px-3 py-1.5 rounded bg-neutral-subtle border border-border text-foreground hover:bg-surface font-medium"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-1.5 rounded bg-primary text-primary-foreground font-semibold hover:bg-primary-hover shadow-xs"
            >
              Import
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
