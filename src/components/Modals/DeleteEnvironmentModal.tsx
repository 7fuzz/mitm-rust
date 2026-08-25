import { useState, useEffect } from 'react';
import { Modal, Button } from '../ui';
import { useTraffic, RepeaterGroup } from '@/hooks/traffic';

interface DeleteEnvironmentModalProps {
  isOpen: boolean;
  envId: string | null;
  envName: string;
  onClose: () => void;
  onConfirmDelete: (envId: string, deleteLinkedCollections: boolean) => Promise<void>;
}

export function DeleteEnvironmentModal({
  isOpen,
  onClose,
  envId,
  envName,
  onConfirmDelete,
}: DeleteEnvironmentModalProps) {
  const { repeaterGroups, manageGroupAssignment } = useTraffic();
  const [linkedGroups, setLinkedGroups] = useState<RepeaterGroup[]>([]);
  const [deleteLinkedCollections, setDeleteLinkedCollections] = useState(false);
  const [loading, setLoading] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    let isMounted = true;
    if (isOpen && envId && envId !== 'default-env-id') {
      setLoading(true);
      setDeleteLinkedCollections(false);

      manageGroupAssignment('get_groups_for_env', undefined, envId)
        .then((assignedIds) => {
          if (!isMounted) return;
          const ids = Array.isArray(assignedIds) ? assignedIds : [];
          const linked = repeaterGroups.filter((g) => ids.includes(g.id));
          setLinkedGroups(linked);
        })
        .catch(console.error)
        .finally(() => {
          if (isMounted) setLoading(false);
        });
    } else {
      setLinkedGroups([]);
      setLoading(false);
    }

    return () => {
      isMounted = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, envId]);

  if (!isOpen || !envId) return null;

  const handleDelete = async () => {
    setDeleting(true);
    try {
      await onConfirmDelete(envId, deleteLinkedCollections);
      onClose();
    } catch (e) {
      console.error('Failed to delete environment:', e);
    } finally {
      setDeleting(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Delete Environment: ${envName}`}
      maxWidth="md"
      footer={
        <div className="flex justify-end gap-3">
          <Button variant="secondary" size="md" onClick={onClose} disabled={deleting}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            size="md"
            onClick={handleDelete}
            disabled={deleting || loading}
            className="px-6"
          >
            {deleting ? 'Deleting...' : 'Delete Environment'}
          </Button>
        </div>
      }
    >
      <div className="p-6 space-y-5">
        <div className="flex items-start gap-3 p-3 bg-rose-500/10 border border-rose-500/30 rounded-lg text-rose-300 text-xs">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="shrink-0 mt-0.5 text-rose-400">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
            <line x1="12" y1="9" x2="12" y2="13"></line>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
          </svg>
          <div>
            <p className="font-bold text-rose-200">Permanently delete environment &quot;{envName}&quot;?</p>
            <p className="mt-1 text-zinc-400 leading-relaxed">
              All environment variables defined under <strong className="text-zinc-200">{envName}</strong> will be permanently destroyed.
            </p>
          </div>
        </div>

        {loading ? (
          <div className="py-6 text-center text-zinc-500 text-xs animate-pulse font-mono">Checking linked collections...</div>
        ) : (
          linkedGroups.length > 0 && (
            <div className="space-y-3 pt-2 border-t border-zinc-800">
              <label className="flex items-start gap-3 p-3 bg-zinc-900 border border-zinc-800 hover:border-amber-500/40 rounded-lg cursor-pointer transition-all">
                <input
                  type="checkbox"
                  checked={deleteLinkedCollections}
                  onChange={(e) => setDeleteLinkedCollections(e.target.checked)}
                  className="accent-amber-500 w-4 h-4 mt-0.5"
                />
                <div className="space-y-1">
                  <span className="text-xs font-bold text-amber-400 uppercase tracking-wider">
                    Delete {linkedGroups.length} Linked Collection(s)
                  </span>
                  <p className="text-[11px] text-zinc-400 leading-relaxed">
                    Check this option to also permanently delete the collections linked to this environment (and all requests inside them).
                  </p>
                </div>
              </label>

              <div className="p-3 bg-zinc-950 border border-zinc-800 rounded-lg max-h-36 overflow-y-auto space-y-1.5 scrollbar-thin scrollbar-thumb-zinc-800">
                <div className="text-[9px] font-mono font-bold uppercase tracking-widest text-zinc-500 mb-1">
                  Linked Collections ({linkedGroups.length})
                </div>
                {linkedGroups.map((g) => (
                  <div key={g.id} className="text-xs font-mono text-zinc-300 flex items-center justify-between py-0.5">
                    <span className="font-bold text-amber-400">📁 {g.name}</span>
                    {deleteLinkedCollections && (
                      <span className="text-[9px] text-rose-400 font-bold uppercase tracking-widest bg-rose-500/10 px-1.5 py-0.5 rounded">
                        Will Be Deleted
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )
        )}
      </div>
    </Modal>
  );
}
