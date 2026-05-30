import React, { useState, useEffect } from 'react';
import { Button, Modal, Input } from '../ui';
import { useTraffic, RepeaterGroup } from '@/hooks/traffic';

interface EnvironmentEditorModalProps {
  isOpen: boolean;
  envId: string | null;
  envName: string;
  onClose: () => void;
}

export function EnvironmentEditorModal({ isOpen, envId, envName, onClose }: EnvironmentEditorModalProps) {
  const { renameEnvironment, manageGroupAssignment, getAllGroups } = useTraffic();
  const [name, setName] = useState(envName);
  const [allGroups, setAllGroups] = useState<RepeaterGroup[]>([]);
  const [assignedGroupIds, setAssignedGroupIds] = useState<string[]>([]);
  const [initialAssignedIds, setInitialAssignedIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (isOpen && envId) {
      setName(envName);
      setLoading(true);
      
      Promise.all([
        getAllGroups(),
        manageGroupAssignment('get_groups_for_env', undefined, envId)
      ]).then(([groups, assignedIds]) => {
        setAllGroups(groups);
        const ids = Array.isArray(assignedIds) ? assignedIds : [];
        setAssignedGroupIds(ids);
        setInitialAssignedIds(ids);
      }).finally(() => setLoading(false));
    }
  }, [isOpen, envId, envName, getAllGroups, manageGroupAssignment]);

  const toggleAssignment = (groupId: string) => {
    setAssignedGroupIds(prev => 
      prev.includes(groupId) ? prev.filter(id => id !== groupId) : [...prev, groupId]
    );
  };

  const selectAll = () => setAssignedGroupIds(allGroups.map(g => g.id));
  const clearAll = () => setAssignedGroupIds([]);

  const handleSave = async () => {
    if (!envId) return;
    setSaving(true);
    
    try {
      // 1. Rename if changed
      if (name.trim() && name !== envName) {
        await renameEnvironment(envId, name);
      }

      // 2. Diff assignments
      const toLink = assignedGroupIds.filter(id => !initialAssignedIds.includes(id));
      const toUnlink = initialAssignedIds.filter(id => !assignedGroupIds.includes(id));

      if (toLink.length > 0 || toUnlink.length > 0) {
        await manageGroupAssignment('bulk', undefined, envId, undefined, {
          links: toLink.map(groupId => ({ groupId, envId })),
          unlinks: toUnlink.map(groupId => ({ groupId, envId }))
        });
      }

      onClose();
    } catch (error) {
      console.error("Failed to save environment changes:", error);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Edit Environment: ${envName}`}
      maxWidth="md"
      footer={
        <div className="flex justify-end gap-3">
          <Button variant="secondary" size="md" onClick={onClose} disabled={saving}>Cancel</Button>
          <Button variant="primary" size="md" onClick={handleSave} className="px-8" disabled={saving || loading}>
            {saving ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      }
    >
      <div className="p-6 space-y-6">
        <div className="space-y-2">
          <label className="text-[10px] text-zinc-500 font-mono uppercase tracking-wider">Environment Name</label>
          <Input 
            value={name} 
            onChange={(e) => setName(e.target.value)} 
            placeholder="Environment Name"
            autoFocus
          />
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between gap-1">
            <div className="flex flex-col gap-1">
              <label className="text-[10px] text-zinc-500 font-mono uppercase tracking-wider">Assigned Collections</label>
              <p className="text-[9px] text-zinc-600">Select which collections should be visible in this environment.</p>
            </div>
            <div className="flex items-center gap-2">
              <button onClick={selectAll} className="text-[9px] font-black uppercase tracking-widest text-sky-500 hover:text-sky-400">Select All</button>
              <div className="w-px h-3 bg-zinc-800"></div>
              <button onClick={clearAll} className="text-[9px] font-black uppercase tracking-widest text-rose-500 hover:text-rose-400">Clear All</button>
            </div>
          </div>

          {loading ? (
            <div className="py-12 text-center text-zinc-600 font-mono text-[10px] animate-pulse">Loading collections...</div>
          ) : (
            <div className="grid grid-cols-1 gap-2 max-h-[300px] overflow-y-auto pr-2 scrollbar-thin scrollbar-thumb-zinc-800">
              {allGroups.map(group => (
                <label key={group.id} className={`flex items-center justify-between p-3 rounded border transition-all cursor-pointer ${assignedGroupIds.includes(group.id) ? 'bg-amber-500/10 border-amber-500/40 text-amber-400' : 'bg-zinc-900/50 border-zinc-800 text-zinc-500 hover:bg-zinc-800/50'}`}>
                  <div className="flex items-center gap-3">
                     <input 
                      type="checkbox" 
                      checked={assignedGroupIds.includes(group.id)} 
                      onChange={() => toggleAssignment(group.id)}
                      className="accent-amber-500 w-4 h-4"
                    />
                    <span className="text-xs font-bold uppercase tracking-widest">{group.name}</span>
                  </div>
                </label>
              ))}
              {allGroups.length === 0 && (
                <div className="py-8 text-center text-zinc-700 text-[10px] font-mono uppercase tracking-widest border border-dashed border-zinc-800 rounded">No collections found</div>
              )}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
