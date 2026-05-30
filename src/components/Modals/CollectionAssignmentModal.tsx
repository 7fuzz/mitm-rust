import { useState, useEffect } from 'react';
import { Button, Modal } from '../ui';
import { useTraffic } from '@/hooks/traffic';

interface CollectionAssignmentModalProps {
  isOpen: boolean;
  groupId: string | null;
  groupName: string;
  onClose: () => void;
}

export function CollectionAssignmentModal({ isOpen, groupId, groupName, onClose }: CollectionAssignmentModalProps) {
  const { environments, manageGroupAssignment } = useTraffic();
  const [assignedEnvIds, setAssignedEnvIds] = useState<string[]>([]);
  const [initialAssignedIds, setInitialAssignedIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (isOpen && groupId) {
      setLoading(true);
      manageGroupAssignment('get_assignments', groupId)
        .then((ids) => {
          const fetchedIds = Array.isArray(ids) ? ids : [];
          setAssignedEnvIds(fetchedIds);
          setInitialAssignedIds(fetchedIds);
        })
        .finally(() => setLoading(false));
    }
  }, [isOpen, groupId, manageGroupAssignment]);

  const toggleAssignment = (envId: string) => {
    setAssignedEnvIds(prev => 
      prev.includes(envId) ? prev.filter(id => id !== envId) : [...prev, envId]
    );
  };

  const selectAll = () => setAssignedEnvIds(environments.map(e => e.id));
  const clearAll = () => setAssignedEnvIds([]);

  const handleSave = async () => {
    if (!groupId) return;
    setSaving(true);
    
    try {
      const toLink = assignedEnvIds.filter(id => !initialAssignedIds.includes(id));
      const toUnlink = initialAssignedIds.filter(id => !assignedEnvIds.includes(id));

      if (toLink.length > 0 || toUnlink.length > 0) {
        await manageGroupAssignment('bulk', groupId, undefined, undefined, {
          links: toLink.map(envId => ({ groupId, envId })),
          unlinks: toUnlink.map(envId => ({ groupId, envId }))
        });
      }

      onClose();
    } catch (error) {
      console.error("Failed to save collection assignments:", error);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Assign Collection: ${groupName}`}
      maxWidth="md"
      footer={
        <div className="flex justify-end gap-3">
          <Button variant="secondary" size="md" onClick={onClose} disabled={saving}>Cancel</Button>
          <Button variant="primary" size="md" onClick={handleSave} className="px-8" disabled={saving || loading}>
            {saving ? 'Saving...' : 'Done'}
          </Button>
        </div>
      }
    >
      <div className="p-6 space-y-4">
        <div className="flex items-center justify-between gap-1">
          <p className="text-[10px] text-zinc-500 font-mono uppercase tracking-wider">
            Select the environments where this collection should be visible.
          </p>
          <div className="flex items-center gap-2">
            <button onClick={selectAll} className="text-[9px] font-black uppercase tracking-widest text-sky-500 hover:text-sky-400">Select All</button>
            <div className="w-px h-3 bg-zinc-800"></div>
            <button onClick={clearAll} className="text-[9px] font-black uppercase tracking-widest text-rose-500 hover:text-rose-400">Clear All</button>
          </div>
        </div>

        {loading ? (
          <div className="py-12 text-center text-zinc-600 font-mono text-[10px] animate-pulse">Loading assignments...</div>
        ) : (
          <div className="grid grid-cols-1 gap-2">
            {environments.map(env => (
              <label key={env.id} className={`flex items-center justify-between p-3 rounded border transition-all cursor-pointer ${assignedEnvIds.includes(env.id) ? 'bg-sky-500/10 border-sky-500/40 text-sky-400' : 'bg-zinc-900/50 border-zinc-800 text-zinc-500 hover:bg-zinc-800/50'}`}>
                <div className="flex items-center gap-3">
                   <input 
                    type="checkbox" 
                    checked={assignedEnvIds.includes(env.id)} 
                    onChange={() => toggleAssignment(env.id)}
                    className="accent-sky-500 w-4 h-4"
                  />
                  <span className="text-xs font-bold uppercase tracking-widest">{env.name}</span>
                </div>
              </label>
            ))}
          </div>
        )}
      </div>
    </Modal>
  );
}
