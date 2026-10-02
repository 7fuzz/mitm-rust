import React, { useState, useRef, useEffect } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Button } from '../../common/ui/Button';
import type { ImportSummary } from '../../../services/tauri/bridge';

export const ProjectImportModal: React.FC = () => {
  const {
    isImportModalOpen,
    setImportModalOpen,
    importProjectJson,
    workspaces,
    activeWorkspaceId,
    importTargetWorkspaceId,
  } = useWorkspaceStore();

  const [importMode, setImportMode] = useState<'new' | 'existing'>('new');
  const [customWsName, setCustomWsName] = useState('');
  const [selectedTargetWsId, setSelectedTargetWsId] = useState(activeWorkspaceId || (workspaces[0]?.id ?? ''));
  const [jsonText, setJsonText] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [copiedError, setCopiedError] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isImportModalOpen) {
      if (importTargetWorkspaceId) {
        setImportMode('existing');
        setSelectedTargetWsId(importTargetWorkspaceId);
      }
    }
  }, [isImportModalOpen, importTargetWorkspaceId]);

  if (!isImportModalOpen) return null;

  const handleClose = () => {
    setImportModalOpen(false);
    setJsonText('');
    setCustomWsName('');
    setSummary(null);
    setErrorMessage(null);
    setImportMode('new');
    setCopiedError(false);
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
      const targetWsId = importMode === 'existing' ? selectedTargetWsId : undefined;
      const wsName = importMode === 'new' ? customWsName.trim() : undefined;

      const result = await importProjectJson(jsonText, targetWsId, wsName);
      setSummary(result);
    } catch (err: any) {
      setErrorMessage(err.toString());
    } finally {
      setIsImporting(false);
    }
  };

  const handleCopyError = () => {
    if (!errorMessage) return;
    navigator.clipboard.writeText(errorMessage);
    setCopiedError(true);
    setTimeout(() => setCopiedError(false), 2000);
  };

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          handleClose();
        }
      }}
      className="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4 font-sans text-xs select-none cursor-pointer"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="bg-surface border border-border rounded-xl shadow-2xl w-full max-w-lg p-5 flex flex-col gap-4 text-foreground cursor-default select-text"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border pb-3 select-none">
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
          /* Drag and drop / Options area */
          <div className="flex flex-col gap-3">
            {/* Target Workspace Option Selector */}
            <div className="bg-background p-3 rounded-lg border border-border space-y-2.5">
              <label className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider block">
                Target Workspace Option
              </label>

              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => setImportMode('new')}
                  className={`flex items-center justify-center gap-2 p-2 rounded-lg border text-xs font-semibold cursor-pointer transition-colors ${
                    importMode === 'new'
                      ? 'bg-primary/10 border-primary text-primary'
                      : 'bg-surface border-border text-muted-foreground hover:text-foreground'
                  }`}
                >
                  <MingCuteIcon name="plus_line" size={14} />
                  <span>Create New Workspace</span>
                </button>

                <button
                  type="button"
                  onClick={() => setImportMode('existing')}
                  className={`flex items-center justify-center gap-2 p-2 rounded-lg border text-xs font-semibold cursor-pointer transition-colors ${
                    importMode === 'existing'
                      ? 'bg-primary/10 border-primary text-primary'
                      : 'bg-surface border-border text-muted-foreground hover:text-foreground'
                  }`}
                >
                  <MingCuteIcon name="folder_block_line" size={14} />
                  <span>Import Into Existing</span>
                </button>
              </div>

              {importMode === 'new' ? (
                <div className="flex flex-col gap-1 pt-1">
                  <label className="text-2xs text-muted-foreground">
                    Workspace Name <span className="text-3xs opacity-70">(Optional - auto-detected if left blank)</span>
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. My New API Workspace"
                    value={customWsName}
                    onChange={(e) => setCustomWsName(e.target.value)}
                    className="bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
                  />
                </div>
              ) : (
                <div className="flex flex-col gap-1 pt-1">
                  <label className="text-2xs text-muted-foreground">Select Target Workspace</label>
                  <select
                    value={selectedTargetWsId}
                    onChange={(e) => setSelectedTargetWsId(e.target.value)}
                    className="bg-surface border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer"
                  >
                    {workspaces.map((ws) => (
                      <option key={ws.id} value={ws.id}>
                        {ws.name} {ws.id === activeWorkspaceId ? '(Current Active)' : ''}
                      </option>
                    ))}
                  </select>
                </div>
              )}
            </div>

            {/* File Upload Drop Zone */}
            <div
              onDragOver={(e) => e.preventDefault()}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current?.click()}
              className="border-2 border-dashed border-border hover:border-primary/60 bg-background/50 hover:bg-neutral-subtle/40 rounded-xl p-5 text-center cursor-pointer transition-colors flex flex-col items-center gap-1.5"
            >
              <MingCuteIcon name="upload_2_line" size={28} className="text-primary opacity-80" />
              <span className="font-semibold text-foreground">Drag and drop JSON file here or click to browse</span>
              <span className="text-2xs text-muted-foreground">Supports Postman v2.1 collections and native workspace specs</span>
              <input
                ref={fileInputRef}
                type="file"
                accept=".json"
                onChange={handleFileUpload}
                className="hidden"
              />
            </div>

            <div className="flex flex-col gap-1">
              <label className="text-2xs font-mono text-muted-foreground">Or paste raw JSON content below:</label>
              <textarea
                value={jsonText}
                onChange={(e) => setJsonText(e.target.value)}
                placeholder='{"info": {"name": "My Collection"}, "item": [...]}'
                rows={4}
                className="w-full bg-background border border-border rounded-lg p-2.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary resize-none"
              />
            </div>

            {errorMessage && (
              <div className="bg-rose-500/10 border border-rose-500/30 text-rose-500 p-3 rounded-lg text-xs font-mono select-text flex flex-col gap-2">
                <div className="flex items-center justify-between font-bold border-b border-rose-500/20 pb-1 text-2xs uppercase tracking-wider">
                  <span className="flex items-center gap-1">
                    <MingCuteIcon name="alert_line" size={14} />
                    Import Error
                  </span>
                  <button
                    type="button"
                    onClick={handleCopyError}
                    className="px-2 py-0.5 bg-rose-500/20 hover:bg-rose-500/30 text-rose-400 font-sans font-semibold rounded text-3xs transition-colors cursor-pointer flex items-center gap-1"
                  >
                    <MingCuteIcon name={copiedError ? 'check_line' : 'copy_line'} size={12} />
                    <span>{copiedError ? 'Copied!' : 'Copy Error'}</span>
                  </button>
                </div>
                <div className="break-words overflow-x-auto max-h-32 no-scrollbar">{errorMessage}</div>
              </div>
            )}

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
              <Button variant="outline" size="sm" onClick={handleClose}>
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                icon="file_import_line"
                onClick={handleExecuteImport}
                disabled={!jsonText.trim() || isImporting}
              >
                {isImporting ? 'Importing...' : 'Import Project'}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
