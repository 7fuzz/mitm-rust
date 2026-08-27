import React, { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { Environment, EnvironmentVariable } from '../../../services/tauri/bridge';
import { VariantManagerModal } from '../../common/VariantManagerModal';

export const WorkspaceSettingsPanel: React.FC = () => {
  const {
    workspaces,
    activeWorkspaceId,
    environmentsList,
    updateWorkspaceDetails,
    deleteWorkspaceById,
    saveEnvironmentVariables,
  } = useWorkspaceStore();

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) || null;

  const [wsName, setWsName] = useState(activeWorkspace?.name || '');
  const [wsDesc, setWsDesc] = useState(activeWorkspace?.description || '');
  const [savedFeedback, setSavedFeedback] = useState(false);
  const [isDeleteModalOpen, setIsDeleteModalOpen] = useState(false);
  const [selectedEnvId, setSelectedEnvId] = useState<string | null>(null);
  const [localVars, setLocalVars] = useState<EnvironmentVariable[]>([]);
  const [managingVarIndex, setManagingVarIndex] = useState<number | null>(null);

  useEffect(() => {
    if (activeWorkspace) {
      setWsName(activeWorkspace.name);
      setWsDesc(activeWorkspace.description || '');
    }
  }, [activeWorkspace]);

  const activeEnv = environmentsList.find((e) => e.id === selectedEnvId) || environmentsList.find((e) => e.isActive) || environmentsList[0] || null;

  useEffect(() => {
    if (activeEnv) {
      setSelectedEnvId(activeEnv.id);
      setLocalVars(activeEnv.variables || []);
    } else {
      setLocalVars([]);
    }
  }, [activeEnv?.id]);

  if (!activeWorkspace) {
    return (
      <div className="p-6 text-center text-muted-foreground italic text-xs">
        No active workspace selected.
      </div>
    );
  }

  const handleSaveWorkspaceInfo = () => {
    updateWorkspaceDetails({
      ...activeWorkspace,
      name: wsName.trim() || 'Untitled Workspace',
      description: wsDesc.trim() || undefined,
      updatedAtMs: Date.now(),
    });
    setSavedFeedback(true);
    setTimeout(() => setSavedFeedback(false), 2000);
  };

  const handleDeleteCurrentWorkspace = async () => {
    if (!activeWorkspace) return;
    await deleteWorkspaceById(activeWorkspace.id);
    setIsDeleteModalOpen(false);
  };

  const handleAddVarRow = () => {
    setLocalVars([...localVars, { key: '', value: '', enabled: true, type: 'default' }]);
  };

  const handleVarChange = (index: number, field: keyof EnvironmentVariable, val: any) => {
    const next = [...localVars];
    next[index] = { ...next[index], [field]: val };
    setLocalVars(next);
  };

  const handleDeleteVarRow = (index: number) => {
    setLocalVars(localVars.filter((_, i) => i !== index));
  };

  const handleSaveEnvironment = async () => {
    if (!activeEnv) return;
    const cleanVars = localVars.filter((v) => v.key.trim().length > 0);
    const updatedEnv: Environment = {
      ...activeEnv,
      variables: cleanVars,
      updatedAtMs: Date.now(),
    };
    await saveEnvironmentVariables(updatedEnv);
  };

  const handleToggleActiveEnvironment = async (env: Environment) => {
    const updatedEnv: Environment = {
      ...env,
      isActive: !env.isActive,
      updatedAtMs: Date.now(),
    };
    await saveEnvironmentVariables(updatedEnv);
  };

  return (
    <div className="flex-1 p-4 overflow-y-auto space-y-6 text-xs text-foreground font-sans">
      {/* Workspace Meta Info Card */}
      <div className="bg-surface border border-border rounded-xl p-4 space-y-3 shadow-2xs">
        <div className="flex items-center justify-between border-b border-border pb-2">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="folder_block_line" size={18} className="text-primary" />
            <span className="font-semibold text-sm">Workspace Details</span>
          </div>

          <div className="flex items-center gap-2">
            {savedFeedback && (
              <span className="text-emerald-500 font-semibold text-xs flex items-center gap-1 animate-fade-in">
                <MingCuteIcon name="check_circle_line" size={14} />
                <span>Saved!</span>
              </span>
            )}
            <button
              onClick={handleSaveWorkspaceInfo}
              className="px-3 py-1 bg-primary hover:bg-primary/90 text-primary-foreground font-semibold rounded text-xs transition-colors cursor-pointer shadow-2xs flex items-center gap-1"
            >
              <MingCuteIcon name="check_line" size={13} />
              <span>Save Workspace Details</span>
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-[11px] font-mono text-muted-foreground">Workspace Name</label>
            <input
              type="text"
              value={wsName}
              onChange={(e) => setWsName(e.target.value)}
              onBlur={handleSaveWorkspaceInfo}
              className="bg-background border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
            />
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-[11px] font-mono text-muted-foreground">Description</label>
            <input
              type="text"
              placeholder="Workspace description..."
              value={wsDesc}
              onChange={(e) => setWsDesc(e.target.value)}
              onBlur={handleSaveWorkspaceInfo}
              className="bg-background border border-border rounded px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary"
            />
          </div>
        </div>
      </div>

      {/* Environment Variables Editor */}
      <div className="bg-surface border border-border rounded-xl p-4 space-y-4 shadow-2xs">
        <div className="flex items-center justify-between border-b border-border pb-2">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="earth_line" size={18} className="text-amber-500" />
            <span className="font-semibold text-sm">Workspace Environment Variables</span>
          </div>

          {activeEnv && (
            <button
              onClick={handleSaveEnvironment}
              className="px-3 py-1 bg-primary hover:bg-primary/90 text-primary-foreground font-semibold rounded text-xs transition-colors cursor-pointer shadow-2xs flex items-center gap-1"
            >
              <MingCuteIcon name="check_line" size={13} />
              <span>Save Variables</span>
            </button>
          )}
        </div>

        <p className="text-muted-foreground text-[11px]">
          Define environment key-value pairs. Reference them anywhere in URLs, Headers, or Request Bodies using macro syntax: <code className="font-mono text-primary font-bold">{`{{variable_name}}`}</code>
        </p>

        {/* Environment Tabs */}
        <div className="flex items-center gap-1 border-b border-border pb-2 overflow-x-auto">
          {environmentsList.map((env) => {
            const isSelected = env.id === selectedEnvId;
            const displayName = typeof env.name === 'string' ? env.name : String(env.name || 'Environment');
            return (
              <button
                key={env.id}
                onClick={() => setSelectedEnvId(env.id)}
                className={`flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-medium cursor-pointer transition-colors ${
                  isSelected
                    ? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
                    : 'bg-background hover:bg-neutral-subtle border border-border text-foreground'
                }`}
              >
                <span>{displayName}</span>
                {env.isActive && (
                  <span className="bg-emerald-500 text-white text-[9px] px-1 py-0.2 rounded font-mono font-bold">
                    ACTIVE
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {/* Selected Environment Controls & Variables Table */}
        {activeEnv ? (
          <div className="space-y-3">
            <div className="flex items-center justify-between bg-header p-2 rounded-lg border border-border">
              <span className="font-medium text-foreground">Set as Active Execution Environment</span>
              <button
                onClick={() => handleToggleActiveEnvironment(activeEnv)}
                className={`px-2.5 py-0.5 rounded text-xs font-semibold transition-colors cursor-pointer ${
                  activeEnv.isActive
                    ? 'bg-emerald-500 text-white'
                    : 'bg-neutral-subtle text-muted-foreground hover:text-foreground'
                }`}
              >
                {activeEnv.isActive ? 'Active' : 'Set Active'}
              </button>
            </div>

            <div className="border border-border rounded-lg overflow-hidden">
              <table className="w-full text-left font-mono text-xs">
                <thead>
                  <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
                    <th className="w-8 px-2 py-1.5 text-center">En</th>
                    <th className="px-3 py-1.5 font-medium">Variable Key</th>
                    <th className="px-3 py-1.5 font-medium">Active Value</th>
                    <th className="w-36 px-2 py-1.5 font-medium">Variant</th>
                    <th className="w-20 px-2 py-1.5 font-medium">Type</th>
                    <th className="w-8 px-2 py-1.5 text-center"></th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border bg-surface">
                  {localVars.length === 0 ? (
                    <tr>
                      <td colSpan={6} className="px-3 py-6 text-center text-muted-foreground italic">
                        No variables defined in this environment yet.
                      </td>
                    </tr>
                  ) : (
                    localVars.map((v, i) => {
                      const keyVal = typeof v.key === 'string' ? v.key : String(v.key || '');
                      const valStr = typeof v.value === 'string' ? v.value : (v.value ? JSON.stringify(v.value) : '');
                      const isEnabled = Boolean(v.enabled);
                      const varType = typeof v.type === 'string' ? v.type : 'default';

                      // Guarantee (auto) variant is always present as variant 0
                      const effectiveVariants = (() => {
                        const list = v.variants && v.variants.length > 0 ? [...v.variants] : [{ name: '(auto)', value: valStr }];
                        const autoIdx = list.findIndex((item) => item.name === '(auto)');
                        if (autoIdx === -1) {
                          list.unshift({ name: '(auto)', value: valStr });
                        } else if (autoIdx > 0) {
                          const [autoItem] = list.splice(autoIdx, 1);
                          list.unshift(autoItem);
                        }
                        return list;
                      })();

                      const activeVariantIdx = typeof v.activeIndex === 'number' && v.activeIndex < effectiveVariants.length ? v.activeIndex : 0;

                      const handleVariantChange = async (newVariantIdx: number) => {
                        if (!effectiveVariants[newVariantIdx]) return;
                        const targetVariant = effectiveVariants[newVariantIdx];
                        const nextVars = [...localVars];
                        nextVars[i] = {
                          ...v,
                          variants: effectiveVariants,
                          activeIndex: newVariantIdx,
                          value: targetVariant.value,
                        };
                        setLocalVars(nextVars);
                        if (activeEnv) {
                          await saveEnvironmentVariables({
                            ...activeEnv,
                            variables: nextVars,
                            updatedAtMs: Date.now(),
                          });
                        }
                      };

                      return (
                        <tr key={i} className="hover:bg-neutral-subtle/50">
                          <td className="px-2 py-1 text-center">
                            <input
                              type="checkbox"
                              checked={isEnabled}
                              onChange={(e) => handleVarChange(i, 'enabled', e.target.checked)}
                              className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer accent-primary"
                            />
                          </td>
                          <td className="px-2 py-1">
                            <input
                              type="text"
                              placeholder="BASE_URL"
                              value={keyVal}
                              onChange={(e) => handleVarChange(i, 'key', e.target.value)}
                              className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                            />
                          </td>
                          <td className="px-2 py-1">
                            <input
                              type={varType === 'secret' ? 'password' : 'text'}
                              placeholder="https://api.example.com"
                              value={valStr}
                              onChange={(e) => handleVarChange(i, 'value', e.target.value)}
                              className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                            />
                          </td>
                          <td className="px-2 py-1">
                            <div className="flex items-center gap-1">
                              <select
                                value={activeVariantIdx}
                                onChange={(e) => handleVariantChange(Number(e.target.value))}
                                className="w-full bg-amber-500/10 text-amber-500 font-bold border border-amber-500/30 rounded px-1.5 py-0.5 text-[11px] focus:outline-none cursor-pointer"
                              >
                                {effectiveVariants.map((variant, idx) => (
                                  <option key={idx} value={idx}>
                                    {variant.name}
                                  </option>
                                ))}
                              </select>
                              <button
                                type="button"
                                onClick={() => setManagingVarIndex(i)}
                                className="p-1 text-amber-500 hover:bg-amber-500/10 rounded transition-colors cursor-pointer shrink-0"
                                title="Manage, Add, or Remove Variants"
                              >
                                <MingCuteIcon name="settings_3_line" size={13} />
                              </button>
                            </div>
                          </td>
                          <td className="px-2 py-1">
                            <select
                              value={varType}
                              onChange={(e) => handleVarChange(i, 'type', e.target.value as any)}
                              className="w-full bg-background border border-border rounded px-1.5 py-0.5 text-[11px] text-foreground focus:outline-none cursor-pointer"
                            >
                              <option value="default">Default</option>
                              <option value="secret">Secret</option>
                            </select>
                          </td>
                        <td className="px-2 py-1 text-center">
                          <button
                            onClick={() => handleDeleteVarRow(i)}
                            className="text-muted-foreground hover:text-rose-500 p-1 rounded transition-colors cursor-pointer"
                          >
                            <MingCuteIcon name="close_line" size={13} />
                          </button>
                        </td>
                      </tr>
                    );
                  })
                  )}
                </tbody>
              </table>
            </div>

            <button
              onClick={handleAddVarRow}
              className="px-3 py-1 rounded bg-background hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs cursor-pointer transition-colors flex items-center gap-1"
            >
              <MingCuteIcon name="plus_line" size={13} />
              <span>Add Variable</span>
            </button>
          </div>
        ) : (
          <div className="py-6 text-center text-muted-foreground italic">
            No environment available.
          </div>
        )}
      </div>

      {/* Danger Zone: Workspace Deletion */}
      <div className="bg-rose-500/5 border border-rose-500/20 rounded-xl p-4 space-y-3">
        <div className="flex items-center gap-2 border-b border-rose-500/20 pb-2 text-rose-500 font-semibold text-xs">
          <MingCuteIcon name="alert_line" size={16} />
          <span>Danger Zone</span>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <h4 className="font-semibold text-foreground">Delete Workspace</h4>
            <p className="text-muted-foreground text-[11px]">
              Permanently remove workspace "{activeWorkspace.name}" and all associated collections, environments, and saved requests.
            </p>
          </div>

          <button
            onClick={() => setIsDeleteModalOpen(true)}
            disabled={workspaces.length <= 1}
            className="px-3.5 py-1.5 bg-rose-600 hover:bg-rose-700 disabled:opacity-40 text-white font-semibold rounded text-xs transition-colors cursor-pointer shadow-2xs shrink-0 flex items-center gap-1.5"
            title={workspaces.length <= 1 ? 'Cannot delete the only workspace' : 'Delete this workspace'}
          >
            <MingCuteIcon name="delete_2_line" size={14} />
            <span>Delete Workspace</span>
          </button>
        </div>
      </div>

      {/* Confirmation Modal */}
      {isDeleteModalOpen && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs z-50 flex items-center justify-center p-4">
          <div className="bg-surface border border-border rounded-xl shadow-2xl p-5 w-full max-w-md text-foreground flex flex-col gap-4">
            <div className="flex items-center gap-2 text-rose-500 font-bold text-sm border-b border-border pb-3">
              <MingCuteIcon name="alert_line" size={20} />
              <span>Confirm Workspace Deletion</span>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Are you sure you want to delete workspace <strong className="text-foreground">"{activeWorkspace.name}"</strong>?
              All collections, environment variables, and saved requests in this workspace will be permanently destroyed.
            </p>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
              <button
                onClick={() => setIsDeleteModalOpen(false)}
                className="px-3.5 py-1.5 rounded bg-header hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground cursor-pointer font-medium"
              >
                Cancel
              </button>
              <button
                onClick={handleDeleteCurrentWorkspace}
                className="px-4 py-1.5 rounded bg-rose-600 hover:bg-rose-700 text-white font-semibold cursor-pointer shadow-2xs"
              >
                Delete Workspace
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Variant Manager Modal */}
      {managingVarIndex !== null && localVars[managingVarIndex] && (
        <VariantManagerModal
          isOpen={managingVarIndex !== null}
          onClose={() => setManagingVarIndex(null)}
          variableKey={typeof localVars[managingVarIndex].key === 'string' ? localVars[managingVarIndex].key : String(localVars[managingVarIndex].key || '')}
          variable={localVars[managingVarIndex]}
          onSave={async (updatedVar) => {
            const nextVars = localVars.map((v, idx) => (idx === managingVarIndex ? updatedVar : v));
            setLocalVars(nextVars);
            if (activeEnv) {
              await saveEnvironmentVariables({
                ...activeEnv,
                variables: nextVars,
                updatedAtMs: Date.now(),
              });
            }
          }}
        />
      )}
    </div>
  );
};
