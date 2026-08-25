import React, { useState, useRef } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { ImportSummary } from '../../../services/tauri/bridge';

export const ProjectImportModal: React.FC = () => {
  const { isImportModalOpen, setImportModalOpen, importProjectJson } = useWorkspaceStore();
  const [jsonText, setJsonText] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (!isImportModalOpen) return null;

  const handleClose = () => {
    setImportModalOpen(false);
    setJsonText('');
    setSummary(null);
    setErrorMessage(null);
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setJsonText(content || '');
    };
    reader.readAsText(file);
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (event) => {
        setJsonText((event.target?.result as string) || '');
      };
      reader.readAsText(file);
    }
  };

  const handleExecuteImport = async () => {
    if (!jsonText.trim()) return;
    setIsImporting(true);
    setErrorMessage(null);
    setSummary(null);

    try {
      const result = await importProjectJson(jsonText);
      setSummary(result);
    } catch (err: any) {
      setErrorMessage(err.toString());
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4 font-sans text-xs select-none">
      <div className="bg-surface border border-border rounded-xl shadow-2xl w-full max-w-lg p-5 flex flex-col gap-4 text-foreground">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border pb-3">
          <div className="flex items-center gap-2 font-semibold text-sm">
            <MingCuteIcon name="file_import_line" size={20} className="text-amber-500" />
            <span>Import JSON Project / Collection</span>
          </div>
          <button
            onClick={handleClose}
            className="text-muted-foreground hover:text-foreground p-1 rounded transition-colors cursor-pointer"
          >
            <MingCuteIcon name="close_line" size={16} />
          </button>
        </div>

        {summary ? (
          /* Import Success Summary */
          <div className="bg-background border border-emerald-500/30 rounded-lg p-4 flex flex-col gap-3">
            <div className="flex items-center gap-2 text-emerald-500 font-bold text-sm">
              <MingCuteIcon name="check_circle_line" size={20} />
              <span>Import Completed Successfully!</span>
            </div>
            <div className="space-y-1 font-mono text-xs text-foreground">
              <div>Workspace: <strong>{summary.workspaceName}</strong></div>
              <div>Environments: <strong>{summary.environmentsImported}</strong></div>
              <div>Collections: <strong>{summary.collectionsImported}</strong></div>
              <div>Requests: <strong>{summary.requestsImported}</strong></div>
            </div>
            <button
              onClick={handleClose}
              className="mt-2 w-full py-1.5 rounded bg-primary text-primary-foreground font-semibold text-xs cursor-pointer shadow-2xs"
            >
              Done
            </button>
          </div>
        ) : (
          /* Drag and drop / Paste area */
          <div className="flex flex-col gap-3">
            <div
              onDragOver={(e) => e.preventDefault()}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current?.click()}
              className="border-2 border-dashed border-border hover:border-primary/60 bg-background/50 hover:bg-neutral-subtle/40 rounded-xl p-6 text-center cursor-pointer transition-colors flex flex-col items-center gap-2"
            >
              <MingCuteIcon name="upload_2_line" size={32} className="text-primary opacity-80" />
              <span className="font-semibold text-foreground">Drag and drop JSON file here or click to browse</span>
              <span className="text-[11px] text-muted-foreground">Supports Postman v2.1 collections and native workspace specs</span>
              <input
                ref={fileInputRef}
                type="file"
                accept=".json"
                onChange={handleFileUpload}
                className="hidden"
              />
            </div>

            <div className="flex flex-col gap-1">
              <label className="text-[11px] font-mono text-muted-foreground">Or paste raw JSON content below:</label>
              <textarea
                value={jsonText}
                onChange={(e) => setJsonText(e.target.value)}
                placeholder='{"info": {"name": "My Collection"}, "item": [...]}'
                rows={5}
                className="w-full bg-background border border-border rounded-lg p-2.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary resize-none"
              />
            </div>

            {errorMessage && (
              <div className="bg-rose-500/10 border border-rose-500/30 text-rose-500 p-2.5 rounded-lg text-xs font-mono">
                {errorMessage}
              </div>
            )}

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
              <button
                onClick={handleClose}
                className="px-3.5 py-1.5 rounded bg-header hover:bg-neutral-subtle text-muted-foreground hover:text-foreground border border-border cursor-pointer font-medium"
              >
                Cancel
              </button>
              <button
                onClick={handleExecuteImport}
                disabled={!jsonText.trim() || isImporting}
                className="px-4 py-1.5 rounded bg-primary hover:bg-primary/90 text-primary-foreground font-semibold cursor-pointer shadow-2xs disabled:opacity-40 flex items-center gap-1.5"
              >
                <MingCuteIcon name="file_import_line" size={14} />
                <span>{isImporting ? 'Importing...' : 'Import Project'}</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
