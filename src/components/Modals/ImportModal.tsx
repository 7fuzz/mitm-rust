import { useState } from 'react';
import { Button, Modal } from '../ui';
import { useTraffic } from '@/hooks/traffic';

interface ImportModalProps {
  isOpen: boolean;
  projectName: string;
  environments: Array<{ id: string; name: string }>;
  groups: Array<{ name: string; target: unknown[] }>;
  onClose: () => void;
  onImport: (options: {
    importAllEnv: boolean;
    selectedEnvIds: string[];
    importAllGroups: boolean;
    selectedGroupNames: string[];
    targetEnvIds: string[];
    smartSync: boolean;
  }) => void;
}

export function ImportModal({ isOpen, projectName, environments, groups, onClose, onImport }: ImportModalProps) {
  const { environments: existingEnvs, activeEnvId } = useTraffic();
  const [selectedEnvIds, setSelectedEnvIds] = useState<string[]>(environments.map(e => e.id));
  const [selectedGroupNames, setSelectedGroupNames] = useState<string[]>(groups.map(g => g.name));
  const [targetEnvIds, setTargetEnvIds] = useState<string[]>(activeEnvId ? [activeEnvId] : []);
  const [smartSync, setSmartSync] = useState(true);

  const toggleEnv = (id: string) => {
    setSelectedEnvIds(prev => 
      prev.includes(id) ? prev.filter(i => i !== id) : [...prev, id]
    );
  };

  const toggleGroup = (name: string) => {
    setSelectedGroupNames(prev => 
      prev.includes(name) ? prev.filter(n => n !== name) : [...prev, name]
    );
  };

  const toggleTargetEnv = (id: string) => {
    setTargetEnvIds(prev =>
      prev.includes(id) ? prev.filter(i => i !== id) : [...prev, id]
    );
  };

  const handleImport = () => {
    onImport({
      importAllEnv: selectedEnvIds.length === environments.length && environments.length > 0,
      selectedEnvIds,
      importAllGroups: selectedGroupNames.length === groups.length && groups.length > 0,
      selectedGroupNames,
      targetEnvIds,
      smartSync
    });
  };

  const hasEnv = environments.length > 0;
  const hasGroups = groups.length > 0;

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Selective Import: ${projectName}`}
      maxWidth="xl"
      footer={
        <div className="flex items-center justify-end gap-3">
          <Button variant="ghost" size="md" onClick={onClose}>Cancel</Button>
          <Button 
            variant="primary" 
            size="md" 
            onClick={handleImport}
            disabled={(selectedEnvIds.length === 0 && selectedGroupNames.length === 0)}
            className="px-8"
          >
            Start Import
          </Button>
        </div>
      }
    >
      <div className="p-6 space-y-8">
        {/* Environments Section */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h4 className="text-[10px] text-amber-500 font-black uppercase tracking-widest">Environments & Variables</h4>
              {hasEnv && (
                <div className="flex items-center gap-2">
                  <div className="flex gap-2 bg-zinc-900 px-2 py-0.5 rounded border border-zinc-800">
                    <button onClick={() => setSelectedEnvIds(environments.map(e => e.id))} className="text-[8px] font-bold text-zinc-500 hover:text-amber-500 uppercase transition-colors">Select All</button>
                    <div className="w-px h-2 bg-zinc-800 my-auto"></div>
                    <button onClick={() => setSelectedEnvIds([])} className="text-[8px] font-bold text-zinc-500 hover:text-rose-500 uppercase transition-colors">Clear All</button>
                  </div>

                  <label className="flex items-center gap-2 px-2 py-0.5 bg-zinc-900 rounded border border-zinc-800 cursor-pointer hover:border-amber-500/50 transition-colors">
                    <input 
                      type="checkbox" 
                      checked={smartSync} 
                      onChange={(e) => setSmartSync(e.target.checked)}
                      className="accent-amber-500 w-3 h-3"
                    />
                    <span className="text-[8px] font-black text-amber-500 uppercase tracking-widest">Smart Link</span>
                  </label>
                </div>
              )}
            </div>
            <div className="text-[9px] text-zinc-500 font-mono">
              {selectedEnvIds.length} / {environments.length} SELECTED
            </div>
          </div>
          {hasEnv && <p className="text-[9px] text-zinc-600 font-mono -mt-2">Smart Link: Automatically make imported collections visible in imported environments.</p>}

          {hasEnv ? (
            <div className="grid grid-cols-2 gap-2">
              {environments.map(env => (
                <label key={env.id} className="flex items-center gap-2 p-2 bg-zinc-900/30 border border-zinc-800 rounded cursor-pointer hover:bg-zinc-900/50 transition-colors">
                  <input 
                    type="checkbox" 
                    checked={selectedEnvIds.includes(env.id)} 
                    onChange={() => toggleEnv(env.id)}
                    className="accent-amber-500"
                  />
                  <span className="text-[10px] font-bold text-zinc-300 uppercase truncate">{env.name}</span>
                </label>
              ))}
            </div>
          ) : (
            <div className="text-[9px] text-zinc-600 uppercase font-bold italic">No environments found in project file.</div>
          )}
        </div>

        {/* Repeater Groups Section */}
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h4 className="text-[10px] text-purple-500 font-black uppercase tracking-widest">Repeater Collections</h4>
              {hasGroups && (
                <div className="flex gap-2 ml-2 bg-zinc-900 px-2 py-0.5 rounded border border-zinc-800">
                  <button onClick={() => setSelectedGroupNames(groups.map(g => g.name))} className="text-[8px] font-bold text-zinc-500 hover:text-purple-400 uppercase transition-colors">Select All</button>
                  <div className="w-px h-2 bg-zinc-800 my-auto"></div>
                  <button onClick={() => setSelectedGroupNames([])} className="text-[8px] font-bold text-zinc-500 hover:text-rose-500 uppercase transition-colors">Clear All</button>
                </div>
              )}
            </div>
            <div className="text-[9px] text-zinc-500 font-mono">
              {selectedGroupNames.length} / {groups.length} SELECTED
            </div>
          </div>

          {hasGroups ? (
            <div className="grid grid-cols-2 gap-2">
              {groups.map(group => (
                <label key={group.name} className="flex items-center gap-2 p-2 bg-zinc-900/30 border border-zinc-800 rounded cursor-pointer hover:bg-zinc-900/50 transition-colors">
                  <input 
                    type="checkbox" 
                    checked={selectedGroupNames.includes(group.name)} 
                    onChange={() => toggleGroup(group.name)}
                    className="accent-purple-500"
                  />
                  <div className="flex flex-col truncate">
                    <span className="text-[10px] font-bold text-zinc-300 uppercase truncate">{group.name}</span>
                    <span className="text-[8px] text-zinc-600 font-mono">{group.target.length} Requests</span>
                  </div>
                </label>
              ))}
            </div>
          ) : (
            <div className="text-[9px] text-zinc-600 uppercase font-bold italic">No requests found in project file.</div>
          )}
        </div>

        {/* Environment Assignment Section */}
        <div className="space-y-4 pt-4 border-t border-zinc-800/50">
          <div className="flex items-center justify-between">
            <h4 className="text-[10px] text-sky-text font-black uppercase tracking-widest">Assign Collections To Existing Environments</h4>
            <div className="text-[9px] text-zinc-500 font-mono">
              {targetEnvIds.length} / {existingEnvs.length} SELECTED
            </div>
          </div>
          <p className="text-[9px] text-zinc-500 font-mono">The imported collections will be visible in these environments.</p>
          
          <div className="grid grid-cols-3 gap-2">
            {existingEnvs.map(env => (
              <label key={env.id} className="flex items-center gap-2 p-2 bg-zinc-900/30 border border-zinc-800 rounded cursor-pointer hover:bg-zinc-900/50 transition-colors">
                <input 
                  type="checkbox" 
                  checked={targetEnvIds.includes(env.id)} 
                  onChange={() => toggleTargetEnv(env.id)}
                  className="accent-sky-500"
                />
                <span className="text-[10px] font-bold text-zinc-300 uppercase truncate">{env.name}</span>
              </label>
            ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}
