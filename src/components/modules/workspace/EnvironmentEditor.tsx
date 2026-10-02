import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { Dialog, Button } from '../../common/ui';
import { EnvironmentList } from './EnvironmentList';
import type { Environment, EnvironmentVariable } from '../../../services/tauri/bridge';
import {
  activeVariantIndex,
  addVariant,
  cycleVariant,
  deleteActiveVariant,
  getVariableVariants,
  newVariable,
  renameActiveVariant,
  setVariableValue,
} from '../../../utils/envVariables';

const SAVE_DELAY_MS = 500;

const cellInputClass =
  'w-full bg-transparent border border-transparent hover:border-border focus:border-primary focus:bg-background rounded px-1.5 py-0.5 text-xs text-foreground font-mono focus:outline-none';

const smallInputClass =
  'min-w-0 flex-1 bg-background border border-primary rounded px-1.5 py-0.5 text-2xs text-foreground font-mono focus:outline-none';

type SaveStatus = 'idle' | 'pending' | 'saving' | 'saved';

export const EnvironmentEditor: React.FC = () => {
  const {
    environmentsList,
    saveEnvironmentVariables,
    createEnvironment,
    duplicateEnvironment,
    renameEnvironment,
    deleteEnvironment,
  } = useWorkspaceStore();

  const [selectedEnvId, setSelectedEnvId] = useState<string | null>(null);
  const [localVars, setLocalVars] = useState<EnvironmentVariable[]>([]);
  const [filter, setFilter] = useState('');
  const [revealed, setRevealed] = useState<Record<number, boolean>>({});
  const [variantEdit, setVariantEdit] = useState<{ index: number; mode: 'add' | 'rename'; name: string } | null>(null);
  const [deletingEnv, setDeletingEnv] = useState<Environment | null>(null);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>('idle');

  const pending = useRef<{ envId: string; vars: EnvironmentVariable[] } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const env =
    environmentsList.find((e) => e.id === selectedEnvId) ||
    environmentsList.find((e) => e.isActive) ||
    environmentsList[0] ||
    null;

  // Reads the environment from the store at save time so a stale copy can't undo "Set as active"
  const flush = useCallback(async () => {
    clearTimeout(timer.current);
    const job = pending.current;
    if (!job) return;
    pending.current = null;
    const target = useWorkspaceStore.getState().environmentsList.find((e) => e.id === job.envId);
    if (!target) return;
    setSaveStatus('saving');
    await saveEnvironmentVariables({
      ...target,
      variables: job.vars.filter((v) => v.key.trim().length > 0),
      updatedAtMs: Date.now(),
    });
    setSaveStatus(pending.current ? 'pending' : 'saved');
  }, [saveEnvironmentVariables]);

  useEffect(() => () => void flush(), [flush]);

  useEffect(() => {
    setSelectedEnvId(env?.id ?? null);
    setLocalVars(env?.variables || []);
    setRevealed({});
    setVariantEdit(null);
    setSaveStatus('idle');
  }, [env?.id]);

  const edit = (vars: EnvironmentVariable[], immediate = false) => {
    if (!env) return;
    setLocalVars(vars);
    pending.current = { envId: env.id, vars };
    setSaveStatus('pending');
    clearTimeout(timer.current);
    if (immediate) flush();
    else timer.current = setTimeout(flush, SAVE_DELAY_MS);
  };

  const editVar = (index: number, next: EnvironmentVariable, immediate = false) =>
    edit(localVars.map((v, i) => (i === index ? next : v)), immediate);

  const switchEnv = async (id: string) => {
    await flush();
    setSelectedEnvId(id);
  };

  const setActive = async (target: Environment) => {
    await flush();
    const latest = useWorkspaceStore.getState().environmentsList.find((e) => e.id === target.id) ?? target;
    await saveEnvironmentVariables({ ...latest, isActive: true, updatedAtMs: Date.now() });
  };

  const selectAfter = async (action: () => Promise<Environment | null>) => {
    await flush();
    const created = await action();
    if (created) setSelectedEnvId(created.id);
  };

  const handleDelete = async () => {
    if (!deletingEnv) return;
    if (pending.current?.envId === deletingEnv.id) {
      clearTimeout(timer.current);
      pending.current = null;
    }
    await deleteEnvironment(deletingEnv.id);
    if (deletingEnv.id === selectedEnvId) setSelectedEnvId(null);
    setDeletingEnv(null);
  };

  const submitVariantEdit = () => {
    if (!variantEdit) return;
    const name = variantEdit.name.trim();
    const v = localVars[variantEdit.index];
    if (name && v) {
      editVar(variantEdit.index, variantEdit.mode === 'add' ? addVariant(v, name) : renameActiveVariant(v, name), true);
    }
    setVariantEdit(null);
  };

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return localVars
      .map((v, index) => ({ v, index }))
      .filter(({ v }) => !q || v.key.toLowerCase().includes(q) || (v.type !== 'secret' && v.value.toLowerCase().includes(q)));
  }, [localVars, filter]);

  const arrowClass =
    'p-0.5 rounded text-amber-500/80 hover:text-amber-500 hover:bg-amber-500/10 cursor-pointer disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent';
  const rowActionClass =
    'p-1 rounded text-muted-foreground opacity-0 group-hover:opacity-100 cursor-pointer transition-opacity';

  return (
    <div className="rounded-lg border border-border bg-surface overflow-hidden flex min-h-[360px]">
      <div className="w-52 shrink-0 border-r border-border">
        <EnvironmentList
          environments={environmentsList}
          selectedId={env?.id ?? null}
          onSelect={switchEnv}
          onCreate={(name) => selectAfter(() => createEnvironment(name))}
          onRename={renameEnvironment}
          onDuplicate={(id) => selectAfter(() => duplicateEnvironment(id))}
          onDelete={setDeletingEnv}
          onSetActive={setActive}
        />
      </div>

      {!env ? (
        <div className="flex-1 flex items-center justify-center text-muted-foreground italic p-6 text-center">
          No environments yet. Add one with + to start defining variables.
        </div>
      ) : (
        <div className="flex-1 min-w-0 flex flex-col">
          <div className="h-10 px-3 bg-header border-b border-border flex items-center gap-3">
            {env.isActive ? (
              <span className="flex items-center gap-1 text-2xs text-emerald-500 font-medium">
                <MingCuteIcon name="check_line" size={13} />
                Active environment
              </span>
            ) : (
              <button
                onClick={() => setActive(env)}
                className="flex items-center gap-1 px-2 py-0.5 rounded border border-border bg-background text-2xs text-foreground hover:bg-neutral-subtle cursor-pointer"
                title="Requests resolve {{variables}} from the active environment"
              >
                <MingCuteIcon name="check_line" size={12} />
                Set as active
              </button>
            )}

            <span className="text-3xs text-muted-foreground flex items-center gap-1">
              {saveStatus === 'saving' || saveStatus === 'pending' ? (
                <>
                  <MingCuteIcon name="loading_line" size={11} className="animate-spin" />
                  Saving...
                </>
              ) : saveStatus === 'saved' ? (
                <>
                  <MingCuteIcon name="check_line" size={11} className="text-emerald-500" />
                  Saved
                </>
              ) : (
                'Changes save automatically'
              )}
            </span>

            <div className="relative ml-auto w-48">
              <MingCuteIcon name="search_line" size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <input
                type="text"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="Filter variables..."
                className="w-full bg-background border border-border rounded pl-7 pr-2 py-1 text-2xs font-mono text-foreground focus:outline-none focus:border-primary"
              />
            </div>
          </div>

          <table className="w-full text-left text-xs table-fixed">
            <thead className="text-3xs uppercase tracking-wider text-muted-foreground">
              <tr className="border-b border-border">
                <th className="w-9 px-2 py-1.5" />
                <th className="w-[24%] px-2 py-1.5 font-semibold">Key</th>
                <th className="w-60 px-2 py-1.5 font-semibold">Variant</th>
                <th className="px-2 py-1.5 font-semibold">Value</th>
                <th className="w-16 px-2 py-1.5" />
              </tr>
            </thead>
            <tbody className="divide-y divide-border/60">
              {visible.map(({ v, index }) => {
                const variants = getVariableVariants(v);
                const active = activeVariantIndex(v);
                const isSecret = v.type === 'secret';
                const isEditingVariant = variantEdit?.index === index;
                return (
                  <tr key={index} className={`group hover:bg-neutral-subtle/40 ${v.enabled ? '' : 'opacity-50'}`}>
                    <td className="px-2 py-1 text-center">
                      <input
                        type="checkbox"
                        checked={Boolean(v.enabled)}
                        onChange={(e) => editVar(index, { ...v, enabled: e.target.checked }, true)}
                        className="h-3.5 w-3.5 cursor-pointer accent-primary"
                        title={v.enabled ? 'Disable' : 'Enable'}
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input
                        type="text"
                        value={v.key}
                        placeholder="variable_name"
                        onChange={(e) => editVar(index, { ...v, key: e.target.value })}
                        className={`${cellInputClass} text-primary font-semibold`}
                      />
                    </td>
                    <td className="px-1 py-1">
                      {isEditingVariant ? (
                        <div className="flex items-center gap-1">
                          <input
                            type="text"
                            autoFocus
                            value={variantEdit.name}
                            onChange={(e) => setVariantEdit({ ...variantEdit, name: e.target.value })}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') submitVariantEdit();
                              if (e.key === 'Escape') setVariantEdit(null);
                            }}
                            onBlur={submitVariantEdit}
                            placeholder={variantEdit.mode === 'add' ? 'new variant name' : 'variant name'}
                            className={smallInputClass}
                          />
                        </div>
                      ) : (
                        <div className="flex items-center gap-0.5">
                          <div className="flex items-center min-w-0 rounded border border-amber-500/30 bg-amber-500/10">
                            <button
                              onClick={() => editVar(index, cycleVariant(v, -1), true)}
                              disabled={variants.length <= 1}
                              className={arrowClass}
                              title="Previous variant"
                            >
                              <MingCuteIcon name="chevron_left_line" size={12} />
                            </button>
                            <span
                              onDoubleClick={() => setVariantEdit({ index, mode: 'rename', name: variants[active].name })}
                              className="px-1 text-2xs font-semibold text-amber-500 truncate cursor-text"
                              title={`${variants[active].name} (${active + 1} of ${variants.length}). Double-click to rename`}
                            >
                              {variants[active].name}
                            </span>
                            {variants.length > 1 && (
                              <span className="text-3xs text-amber-500/70 font-mono pr-0.5 shrink-0">
                                {active + 1}/{variants.length}
                              </span>
                            )}
                            <button
                              onClick={() => editVar(index, cycleVariant(v, 1), true)}
                              disabled={variants.length <= 1}
                              className={arrowClass}
                              title="Next variant"
                            >
                              <MingCuteIcon name="chevron_right_line" size={12} />
                            </button>
                          </div>
                          <button
                            onClick={() => setVariantEdit({ index, mode: 'add', name: '' })}
                            className={`${rowActionClass} hover:text-amber-500`}
                            title="Add a variant (it becomes the active one)"
                          >
                            <MingCuteIcon name="plus_line" size={12} />
                          </button>
                          {variants.length > 1 && (
                            <button
                              onClick={() => editVar(index, deleteActiveVariant(v), true)}
                              className={`${rowActionClass} hover:text-rose-500`}
                              title={`Delete variant "${variants[active].name}"`}
                            >
                              <MingCuteIcon name="close_line" size={12} />
                            </button>
                          )}
                        </div>
                      )}
                    </td>
                    <td className="px-1 py-1">
                      <div className="flex items-center gap-1">
                        <input
                          type={isSecret && !revealed[index] ? 'password' : 'text'}
                          value={v.value}
                          placeholder={variants.length > 1 ? `value for "${variants[active].name}"` : 'value'}
                          onChange={(e) => editVar(index, setVariableValue(v, e.target.value))}
                          className={cellInputClass}
                        />
                        {isSecret && (
                          <button
                            onClick={() => setRevealed((r) => ({ ...r, [index]: !r[index] }))}
                            className="p-1 rounded text-muted-foreground hover:text-foreground cursor-pointer shrink-0"
                            title={revealed[index] ? 'Hide value' : 'Show value'}
                          >
                            <MingCuteIcon name={revealed[index] ? 'eye_close_line' : 'eye_line'} size={12} />
                          </button>
                        )}
                      </div>
                    </td>
                    <td className="px-1 py-1">
                      <div className="flex items-center justify-end gap-0.5">
                        <button
                          onClick={() => editVar(index, { ...v, type: isSecret ? 'default' : 'secret' }, true)}
                          className={
                            isSecret
                              ? 'p-1 rounded text-amber-500 cursor-pointer'
                              : `${rowActionClass} hover:text-foreground`
                          }
                          title={isSecret ? 'Secret (masked). Click to make it a plain value' : 'Mark as secret (masked)'}
                        >
                          <MingCuteIcon name={isSecret ? 'lock_line' : 'unlock_line'} size={12} />
                        </button>
                        <button
                          onClick={() => edit(localVars.filter((_, i) => i !== index), true)}
                          className={`${rowActionClass} hover:text-rose-500`}
                          title="Delete variable"
                        >
                          <MingCuteIcon name="delete_2_line" size={12} />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
              {visible.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-3 py-6 text-center text-muted-foreground italic">
                    {filter ? 'No variables match the filter' : 'No variables yet'}
                  </td>
                </tr>
              )}
            </tbody>
          </table>

          <button
            onClick={() => {
              setFilter('');
              setLocalVars((vars) => [...vars, newVariable()]);
            }}
            className="w-full flex items-center gap-1.5 px-3 py-2 border-t border-border text-2xs text-muted-foreground hover:text-foreground hover:bg-neutral-subtle/40 cursor-pointer text-left"
          >
            <MingCuteIcon name="plus_line" size={12} />
            Add variable
          </button>
        </div>
      )}

      <Dialog
        isOpen={!!deletingEnv}
        onClose={() => setDeletingEnv(null)}
        title="Delete environment"
        size="md"
        footer={
          <>
            <Button variant="subtle" onClick={() => setDeletingEnv(null)}>
              Cancel
            </Button>
            <Button variant="danger" onClick={handleDelete}>
              Delete environment
            </Button>
          </>
        }
      >
        <p className="text-xs text-muted-foreground leading-relaxed">
          Delete <strong className="text-foreground">"{deletingEnv?.name}"</strong> and its{' '}
          {deletingEnv?.variables?.length ?? 0} variables?
          {deletingEnv?.isActive && ' It is the active environment, so the oldest remaining one becomes active.'}
        </p>
      </Dialog>
    </div>
  );
};
