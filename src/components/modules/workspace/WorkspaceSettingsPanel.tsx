import React, { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import type { Environment, EnvironmentVariable } from '../../../services/tauri/bridge';

export const WorkspaceSettingsPanel: React.FC = () => {
  const {
    workspaces,
    activeWorkspaceId,
    environmentsList,
    updateWorkspaceDetails,
    saveEnvironmentVariables,
  } = useWorkspaceStore();

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) || null;

  const [wsName, setWsName] = useState(activeWorkspace?.name || '');
  const [wsDesc, setWsDesc] = useState(activeWorkspace?.description || '');
  const [selectedEnvId, setSelectedEnvId] = useState<string | null>(null);
  const [localVars, setLocalVars] = useState<EnvironmentVariable[]>([]);

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
    <div className="flex-1 p-4 overflow-y-auto space-y-6 text-xs text-foreground select-none font-sans">
      {/* Workspace Meta Info Card */}
      <div className="bg-surface border border-border rounded-xl p-4 space-y-3 shadow-2xs">
        <div className="flex items-center gap-2 border-b border-border pb-2">
          <MingCuteIcon name="folder_block_line" size={18} className="text-primary" />
          <span className="font-semibold text-sm">Workspace Details</span>
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
                <span>{env.name}</span>
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
                    <th className="px-3 py-1.5 font-medium">Value</th>
                    <th className="w-24 px-2 py-1.5 font-medium">Type</th>
                    <th className="w-8 px-2 py-1.5 text-center"></th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border bg-surface">
                  {localVars.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="px-3 py-6 text-center text-muted-foreground italic">
                        No variables defined in this environment yet.
                      </td>
                    </tr>
                  ) : (
                    localVars.map((v, i) => (
                      <tr key={i} className="hover:bg-neutral-subtle/50">
                        <td className="px-2 py-1 text-center">
                          <input
                            type="checkbox"
                            checked={v.enabled}
                            onChange={(e) => handleVarChange(i, 'enabled', e.target.checked)}
                            className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer accent-primary"
                          />
                        </td>
                        <td className="px-2 py-1">
                          <input
                            type="text"
                            placeholder="BASE_URL"
                            value={v.key}
                            onChange={(e) => handleVarChange(i, 'key', e.target.value)}
                            className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                          />
                        </td>
                        <td className="px-2 py-1">
                          <input
                            type={v.type === 'secret' ? 'password' : 'text'}
                            placeholder="https://api.example.com"
                            value={v.value}
                            onChange={(e) => handleVarChange(i, 'value', e.target.value)}
                            className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground font-mono focus:outline-none focus:border-primary"
                          />
                        </td>
                        <td className="px-2 py-1">
                          <select
                            value={v.type}
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
                    ))
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
    </div>
  );
};
