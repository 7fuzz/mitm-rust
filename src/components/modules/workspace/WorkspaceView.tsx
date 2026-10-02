import React, { useEffect, useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { useUiPref } from '../../../stores/useUiPrefsStore';
import type { Workspace } from '../../../services/tauri/bridge';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Dialog, Button } from '../../common/ui';
import { SettingsGroup, SettingsRow, SettingsSection, settingsButtonClass } from '../settings/SettingsSection';
import { WorkspaceSidebar } from './WorkspaceSidebar';
import { EnvironmentEditor } from './EnvironmentEditor';
import { ProjectImportModal } from './ProjectImportModal';

type Notice = { type: 'success' | 'error'; message: string };

export const WorkspaceView: React.FC = () => {
  const {
    initStore,
    workspaces,
    activeWorkspaceId,
    updateWorkspaceDetails,
    deleteWorkspaceById,
    exportWorkspaceToFile,
    openImportModalForWorkspace,
  } = useWorkspaceStore();

  const workspace = workspaces.find((w) => w.id === activeWorkspaceId) || null;
  const [sidebarOpen, setSidebarOpen] = useUiPref('workspace.sidebarOpen');

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [deleting, setDeleting] = useState<Workspace | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);

  useEffect(() => {
    initStore();
  }, [initStore]);

  useEffect(() => {
    setName(workspace?.name ?? '');
    setDescription(workspace?.description ?? '');
  }, [workspace?.id, workspace?.name, workspace?.description]);

  const flash = (next: Notice) => {
    setNotice(next);
    setTimeout(() => setNotice(null), next.type === 'success' ? 3000 : 6000);
  };

  const saveDetails = () => {
    if (!workspace) return;
    const nextName = name.trim() || workspace.name;
    const nextDescription = description.trim() || undefined;
    setName(nextName);
    if (nextName === workspace.name && nextDescription === (workspace.description || undefined)) return;
    updateWorkspaceDetails({ ...workspace, name: nextName, description: nextDescription, updatedAtMs: Date.now() });
  };

  const handleExport = async (workspaceId: string) => {
    setIsExporting(true);
    try {
      const result = await exportWorkspaceToFile(workspaceId);
      if (result.success && result.filePath) flash({ type: 'success', message: `Exported to ${result.filePath}` });
      else if (result.error) flash({ type: 'error', message: `Export failed: ${result.error}` });
    } finally {
      setIsExporting(false);
    }
  };

  const handleDelete = async () => {
    if (!deleting) return;
    await deleteWorkspaceById(deleting.id);
    setDeleting(null);
  };

  const blurOnEnter = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') e.currentTarget.blur();
  };

  return (
    <div className="h-full flex bg-background overflow-hidden text-xs">
      {sidebarOpen && (
        <div className="w-60 shrink-0">
          <WorkspaceSidebar onRequestDelete={setDeleting} onExport={handleExport} onHide={() => setSidebarOpen(false)} />
        </div>
      )}

      <main className="flex-1 min-w-0 overflow-y-auto">
        {!workspace ? (
          <div className="h-full flex flex-col items-center justify-center gap-3 text-muted-foreground italic">
            <span>No workspace selected</span>
            {!sidebarOpen && (
              <button onClick={() => setSidebarOpen(true)} className={settingsButtonClass.secondary}>
                <MingCuteIcon name="chevron_right_line" size={14} />
                <span>Show workspace list</span>
              </button>
            )}
          </div>
        ) : (
          <div className="px-6 py-5 space-y-6">
            <header className="flex items-start gap-4">
              {!sidebarOpen && (
                <button
                  onClick={() => setSidebarOpen(true)}
                  className="mt-1 p-1.5 rounded border border-border bg-background text-muted-foreground hover:text-foreground cursor-pointer shrink-0"
                  title={`Show workspace list (${workspaces.length})`}
                >
                  <MingCuteIcon name="chevron_right_line" size={14} />
                </button>
              )}
              <div className="flex-1 min-w-0 space-y-0.5">
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  onBlur={saveDetails}
                  onKeyDown={blurOnEnter}
                  className="w-full bg-transparent border border-transparent hover:border-border focus:border-primary rounded px-1.5 -mx-1.5 py-0.5 text-lg font-semibold text-foreground focus:outline-none"
                  title="Click to rename"
                />
                <input
                  type="text"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  onBlur={saveDetails}
                  onKeyDown={blurOnEnter}
                  placeholder="Add a description..."
                  className="w-full bg-transparent border border-transparent hover:border-border focus:border-primary rounded px-1.5 -mx-1.5 py-0.5 text-xs text-muted-foreground focus:outline-none"
                />
              </div>
              <div className="flex items-center gap-2 shrink-0 pt-1">
                <button onClick={() => openImportModalForWorkspace(workspace.id)} className={settingsButtonClass.secondary}>
                  <MingCuteIcon name="file_import_line" size={14} />
                  <span>Import into</span>
                </button>
                <button onClick={() => handleExport(workspace.id)} disabled={isExporting} className={settingsButtonClass.secondary}>
                  <MingCuteIcon name="download_line" size={14} />
                  <span>{isExporting ? 'Exporting...' : 'Export'}</span>
                </button>
              </div>
            </header>

            {notice && (
              <div
                className={`flex items-center gap-2 px-3 py-2 rounded-lg border font-mono text-2xs ${
                  notice.type === 'success'
                    ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-500'
                    : 'bg-rose-500/10 border-rose-500/30 text-rose-500'
                }`}
              >
                <MingCuteIcon name={notice.type === 'success' ? 'check_line' : 'alert_line'} size={13} className="shrink-0" />
                <span className="break-all">{notice.message}</span>
              </div>
            )}

            <SettingsSection
              title="Environments"
              description={
                <>
                  Variables are referenced as <code className="font-mono text-primary">{'{{name}}'}</code> in URLs, headers,
                  and bodies, and resolve from the active environment.
                </>
              }
            >
              <EnvironmentEditor key={workspace.id} />
            </SettingsSection>

            <SettingsGroup label="Danger zone" tone="danger">
              <SettingsRow
                label="Delete workspace"
                description={
                  workspaces.length <= 1
                    ? 'The only workspace cannot be deleted.'
                    : 'Removes its collections, environments, and saved requests for good.'
                }
              >
                <button
                  onClick={() => setDeleting(workspace)}
                  disabled={workspaces.length <= 1}
                  className={settingsButtonClass.danger}
                >
                  <MingCuteIcon name="delete_2_line" size={14} />
                  <span>Delete</span>
                </button>
              </SettingsRow>
            </SettingsGroup>
          </div>
        )}
      </main>

      <Dialog
        isOpen={!!deleting}
        onClose={() => setDeleting(null)}
        title="Delete workspace"
        size="md"
        footer={
          <>
            <Button variant="subtle" onClick={() => setDeleting(null)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={handleDelete}>
              Delete workspace
            </Button>
          </>
        }
      >
        <p className="text-xs text-muted-foreground leading-relaxed">
          Delete <strong className="text-foreground">"{deleting?.name}"</strong>? Its collections, environment variables,
          and saved requests are permanently removed.
        </p>
      </Dialog>

      <ProjectImportModal />
    </div>
  );
};
