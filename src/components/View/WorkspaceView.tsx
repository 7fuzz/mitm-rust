import { useState, useImperativeHandle, forwardRef } from 'react';
import { useTraffic, RepeaterRequest } from '@/hooks/traffic';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { ConfirmModal, PromptModal, MultiGroupExportModal, ImportModal } from '../Modals';
import { EnvironmentsSection } from '../modules/workspace/EnvironmentsSection';
import { CollectionsSection } from '../modules/workspace/CollectionsSection';
import { ReplacementsSection } from '../modules/workspace/ReplacementsSection';
import { Button } from '../ui/Button';
import { useNotification } from '../ui/NotificationProvider';

export type WorkspaceTab = 'env' | 'collections' | 'replacements' | 'sync';

export interface WorkspaceViewHandle {
  switchTab: (tab: WorkspaceTab) => void;
}

export const WorkspaceView = forwardRef<WorkspaceViewHandle, object>((_, ref) => {
  const { notify } = useNotification();
  const {
    uiLayout, updateUILayout,
    variables, activeEnvId,
    environments,
    repeaterGroups,
    importPostman, importProject, finalizeImport,
    syncAll, syncStatus
  } = useTraffic();

  const [activeTab, setActiveTab] = useState<WorkspaceTab>('env');

  useImperativeHandle(ref, () => ({
    switchTab: (tab: WorkspaceTab) => setActiveTab(tab)
  }));

  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);

  const [promptConfig, setPromptConfig] = useState({ isOpen: false, title: '', initialValue: '', action: (_val: string) => { } });
  const openPrompt = (title: string, initialValue: string, action: (val: string) => void) => setPromptConfig({ isOpen: true, title, initialValue, action });
  const closePrompt = () => setPromptConfig(prev => ({ ...prev, isOpen: false }));

  const [confirmConfig, setConfirmConfig] = useState({ isOpen: false, title: '', message: '', action: () => { } });
  const openConfirm = (title: string, message: string, action: () => void) => setConfirmConfig({ isOpen: true, title, message, action });

  const [exportModalOpen, setExportModalOpen] = useState(false);

  // Import Selective State
  const [importData, setImportData] = useState<{
    name?: string;
    all_environments?: Array<{ id: string; name: string }>;
    test_cases?: Array<{ name: string; target: unknown[] }>;
    placeholders?: Record<string, string>;
    __isLegacy?: boolean;
  } | null>(null);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);

  const handleImportFileLoaded = (data: {
    all_environments?: Array<{ id: string; name: string }>;
    placeholders?: Record<string, string>;
    __isLegacy?: boolean;
  }) => {
    // Detect legacy placeholders and present them as a virtual environment for selective import
    if ((!data.all_environments || data.all_environments.length === 0) && data.placeholders && Object.keys(data.placeholders).length > 0) {
      data.__isLegacy = true;
      data.all_environments = [{ id: 'legacy-env', name: 'Variables (Legacy Project Format)' }];
    }
    setImportData(data);
    setIsImportModalOpen(true);
  };

  const handleExport = async (selectedGroupIds: string[], projectName: string) => {
    try {
      const fetchPromises = selectedGroupIds.map(gid => fetch(`/api/repeater-db?groupId=${gid}`).then(r => r.json()));
      const results = await Promise.all(fetchPromises);
      const flattenedRequests: RepeaterRequest[] = results.flat();

      if (flattenedRequests.length === 0) return alert('No requests found in selected groups.');

      // --- FULL ENVIRONMENT EXPORT ---
      const exportEnvironments = environments;
      const allEnvVariables = variables; // All variables for all environments

      const placeholders: Record<string, string> = {};
      variables.filter(v => v.environmentId === activeEnvId).forEach(v => {
        if (v.name.trim()) {
          const activeVal = v.values[v.activeIndex] || v.values[0];
          placeholders[v.name.trim()] = activeVal ? activeVal.value : '';
        }
      });

      const splitUrl = (urlStr: string) => {
        let baseUrl = '';
        let endpoint = urlStr;
        const params: Record<string, string> = {};

        if (urlStr.startsWith('{{')) {
          const endOfVar = urlStr.indexOf('}}');
          if (endOfVar !== -1) {
            const firstSlash = urlStr.indexOf('/', endOfVar);
            if (firstSlash !== -1) {
              baseUrl = urlStr.substring(0, firstSlash);
              endpoint = urlStr.substring(firstSlash);
            } else { baseUrl = urlStr; endpoint = ''; }
          }
        } else {
          try {
            const u = new URL(urlStr);
            baseUrl = u.origin;
            endpoint = u.pathname;
          } catch (_e) { /* fallback */ }
        }

        try {
          const searchIdx = endpoint.indexOf('?');
          if (searchIdx !== -1) {
            const search = endpoint.substring(searchIdx);
            endpoint = endpoint.substring(0, searchIdx);
            const sp = new URLSearchParams(search);
            sp.forEach((v, k) => { params[k] = v; });
          }
        } catch (_e) { /* ignore */ }
        return { baseUrl, endpoint, params };
      };

      const headerCounts: Record<string, Record<string, number>> = {};
      flattenedRequests.forEach(req => {
        (req.headers || []).forEach(([k, v]) => {
          const key = (k as string).toLowerCase();
          if (!headerCounts[key]) headerCounts[key] = {};
          headerCounts[key][v as string] = (headerCounts[key][v as string] || 0) + 1;
        });
      });

      const globalHeader: Record<string, string> = {};
      const threshold = Math.max(1, flattenedRequests.length * 0.6);
      Object.entries(headerCounts).forEach(([key, vals]) => {
        Object.entries(vals).forEach(([v, count]) => {
          if (count >= threshold) {
            const sampleReq = flattenedRequests.find(r => (r.headers || []).some(([hk]) => hk.toLowerCase() === key));
            const originalKey = (sampleReq?.headers || []).find(([hk]) => hk.toLowerCase() === key)?.[0] || key;
            globalHeader[originalKey] = v;
          }
        });
      });

      const test_cases = selectedGroupIds.map(gid => {
        const groupReqs = flattenedRequests.filter(r => (r.groupId || 'null') === gid);
        if (groupReqs.length === 0) return null;

        const groupName = gid === 'null' ? 'Default' : (repeaterGroups.find(g => g.id === gid)?.name || 'Unknown Group');

        const baseUrls = groupReqs.map(r => splitUrl(r.url).baseUrl);
        const mostCommonBase = baseUrls.sort((a, b) => baseUrls.filter(v => v === a).length - baseUrls.filter(v => v === b).length).pop() || '{{apiUrl}}';

        const targets = groupReqs.map(req => {
          const { baseUrl, endpoint, params } = splitUrl(req.url);
          let parsedBody = req.body;
          try {
            if (req.body && (req.body.startsWith('{') || req.body.startsWith('['))) {
              parsedBody = JSON.parse(req.body);
            }
          } catch (_e) { /* keep as string */ }

          const localHeaders: Record<string, string | null> = {};
          (req.headers || []).forEach(([k, v]) => {
            const gk = Object.keys(globalHeader).find(key => key.toLowerCase() === k.toLowerCase());
            if (!gk || globalHeader[gk] !== v) {
              localHeaders[k] = v;
            }
          });
          Object.keys(globalHeader).forEach(gk => {
            if (!(req.headers || []).some(([rk]) => rk.toLowerCase() === gk.toLowerCase())) {
              localHeaders[gk] = null;
            }
          });

          return {
            name: req.name,
            endpoint: baseUrl === mostCommonBase ? endpoint : (baseUrl + endpoint),
            method: req.method,
            header: Object.keys(localHeaders).length > 0 ? localHeaders : undefined,
            params: Object.keys(params).length > 0 ? params : undefined,
            body: req.method !== 'GET' ? parsedBody : undefined,
            extract: req.extract || {}
          };
        });

        return {
          name: groupName,
          url: mostCommonBase,
          target: targets
        };
      }).filter((tc): tc is NonNullable<typeof tc> => tc !== null);

      const exportData = {
        name: projectName,
        url: (test_cases[0] as { url: string })?.url || '{{apiUrl}}',
        header: globalHeader,
        placeholders,
        all_environments: exportEnvironments,
        all_variables: allEnvVariables,
        test_cases
      };

      const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${projectName.replace(/[^a-z0-9]/gi, '_').toLowerCase()}_export.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

    } catch (_err) { alert('Export failed: ' + _err); }
  };

  return (
    <WorkspaceLayout
      uiLayout={uiLayout}
      onUpdateLayout={updateUILayout}
      listComponent={() => (
        <div className="flex flex-col p-2 gap-1 h-full">
          <div className="px-3 py-2 text-[9px] font-black tracking-widest text-zinc-600 uppercase mb-2">Workspace Setup</div>
          <button
            onClick={() => setActiveTab('env')}
            className={`flex items-center text-left px-3 py-2.5 rounded text-[11px] font-bold tracking-wider transition-all border ${activeTab === 'env' ? 'bg-amber-500/10 border-amber-500/30 text-amber-400' : 'bg-transparent border-transparent text-zinc-400 hover:bg-zinc-900'}`}
          >
            Environments & Variables
          </button>
          <button
            onClick={() => setActiveTab('collections')}
            className={`flex items-center text-left px-3 py-2.5 rounded text-[11px] font-bold tracking-wider transition-all border ${activeTab === 'collections' ? 'bg-purple-500/10 border-purple-500/40 text-purple-400' : 'bg-transparent border-transparent text-zinc-400 hover:bg-zinc-900'}`}
          >
            Repeater Collections
          </button>
          <button
            onClick={() => setActiveTab('replacements')}
            className={`flex items-center text-left px-3 py-2.5 rounded text-[11px] font-bold tracking-wider transition-all border ${activeTab === 'replacements' ? 'bg-rose-highlight-bg border-rose-highlight-border text-rose-text' : 'bg-transparent border-transparent text-zinc-400 hover:bg-zinc-900'}`}
          >
            Repeater Replacements
          </button>
          <button
            onClick={() => setActiveTab('sync')}
            className={`flex items-center text-left px-3 py-2.5 rounded text-[11px] font-bold tracking-wider transition-all border ${activeTab === 'sync' ? 'bg-sky-highlight-bg border-sky-highlight-border text-sky-text' : 'bg-transparent border-transparent text-zinc-400 hover:bg-zinc-900'}`}
          >
            Database & Sync
          </button>
        </div>
      )}
      toolbarLeft={
        <div className="flex items-center px-4">
          <span className="text-[12px] font-black uppercase tracking-[0.2em] text-zinc-300">
            Workspace_Management / {activeTab === 'env' ? 'Environments' : activeTab === 'collections' ? 'Collections' : activeTab === 'replacements' ? 'Replacements' : 'Database & Sync'}
          </span>
        </div>
      }

      toolbarRight={
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={importPostman} className="text-zinc-500 hover:text-sky-text">Import PM</Button>
          <Button variant="ghost" size="sm" onClick={() => importProject(handleImportFileLoaded)} className="text-zinc-500 hover:text-sky-text">Import Project</Button>
          <Button variant="ghost" size="sm" onClick={() => setExportModalOpen(true)} className="text-zinc-500 hover:text-amber-400">Export</Button>
        </div>
      }
      mainContent={() => (
        <div className="w-full max-w-5xl mx-auto py-6 space-y-10">
          {activeTab === 'env' && <EnvironmentsSection openPrompt={openPrompt} openConfirm={openConfirm} />}
          {activeTab === 'collections' && <CollectionsSection selectedGroupId={selectedGroupId} setSelectedGroupId={setSelectedGroupId} openPrompt={openPrompt} openConfirm={openConfirm} />}
          {activeTab === 'replacements' && <ReplacementsSection />}
          {activeTab === 'sync' && (
             <div className="space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
               <div className="bg-zinc-950 border border-zinc-800 rounded-xl p-6 shadow-2xl">
                 <div className="flex items-center justify-between mb-8">
                   <div className="space-y-1">
                     <h2 className="text-xl font-black tracking-tighter text-zinc-100 flex items-center gap-3">
                       <span className="w-2 h-8 bg-sky-500 rounded-full"></span>
                       DATABASE_SYNCHRONIZATION
                     </h2>
                     <p className="text-xs text-zinc-500 font-mono tracking-wide uppercase opacity-70">Manage internal SQLite state and manual data fetches</p>
                   </div>
                   <Button 
                     variant="sky" 
                     size="lg" 
                     onClick={() => syncAll()} 
                     disabled={syncStatus.is_syncing}
                     className="min-w-40"
                   >
                     {syncStatus.is_syncing ? 'SYNCING...' : 'MANUAL_SYNC'}
                   </Button>
                 </div>

                 <div className="grid grid-cols-3 gap-6">
                   <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
                     <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Sync Status</span>
                     <div className="flex items-center gap-2">
                       <div className={`w-2 h-2 rounded-full ${syncStatus.is_syncing ? 'bg-amber-500 animate-pulse' : syncStatus.error ? 'bg-rose-500' : 'bg-emerald-500'}`}></div>
                       <span className={`text-xs font-black uppercase tracking-wider ${syncStatus.is_syncing ? 'text-amber-500' : syncStatus.error ? 'text-rose-500' : 'text-emerald-500'}`}>
                         {syncStatus.is_syncing ? 'Synchronizing' : syncStatus.error ? 'Connection Error' : 'All Systems Nominal'}
                       </span>
                     </div>
                   </div>
                   <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
                     <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Last Synchronized</span>
                     <span className="text-xs font-mono text-zinc-400 block">
                       {syncStatus.last_sync ? new Date(syncStatus.last_sync).toLocaleString() : 'NEVER'}
                     </span>
                   </div>
                   <div className="bg-zinc-900/50 border border-zinc-800/50 p-4 rounded-lg space-y-2">
                     <span className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block">Local Records</span>
                     <span className="text-xs font-mono text-zinc-400 block">
                       {(useTraffic() as any).traffic?.length || 0} History Items
                     </span>
                   </div>
                 </div>

                 {syncStatus.error && (
                   <div className="mt-6 p-4 bg-rose-500/10 border border-rose-500/30 rounded-lg flex items-center gap-4 text-rose-400">
                     <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                     <div className="flex-1">
                       <span className="text-[10px] font-black uppercase tracking-widest block mb-0.5">Critical Synchronizer Failure</span>
                       <span className="text-xs font-mono opacity-80">{syncStatus.error}</span>
                     </div>
                     <Button variant="destructive" size="xs" onClick={() => syncAll()}>Retry Sync</Button>
                   </div>
                 )}
               </div>

               <div className="grid grid-cols-2 gap-8">
                 <div className="p-6 bg-zinc-950 border border-zinc-800 rounded-xl space-y-4">
                   <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-500 flex items-center gap-2">
                     <span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span>
                     Persistence_Information
                   </h3>
                   <div className="space-y-3">
                     {[
                       { label: 'Database Type', value: 'SQLite 3.x (Local)' },
                       { label: 'Storage Mode', value: 'Application Data Directory' },
                       { label: 'Sync Engine', value: 'Rust / Tauri IPC' },
                       { label: 'Auto-Sync', value: 'On Event (Captured)' }
                     ].map(item => (
                       <div key={item.label} className="flex items-center justify-between py-2 border-b border-zinc-900 last:border-0">
                         <span className="text-[11px] text-zinc-500 font-medium">{item.label}</span>
                         <span className="text-[11px] text-zinc-400 font-mono">{item.value}</span>
                       </div>
                     ))}
                   </div>
                 </div>

                 <div className="p-6 bg-zinc-950 border border-zinc-800 rounded-xl space-y-4">
                   <h3 className="text-[10px] font-black uppercase tracking-[0.3em] text-zinc-500 flex items-center gap-2">
                     <span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span>
                     Advanced_Operations
                   </h3>
                   <div className="grid grid-cols-1 gap-3">
                     <Button variant="secondary" size="sm" className="justify-start px-4 h-11" onClick={() => notify.info('Maintenance: Vacuuming database...')}>
                       <span className="flex-1 text-left">Vacuum Database</span>
                       <span className="text-[8px] opacity-40 font-mono uppercase tracking-widest">Optimizes Disk Space</span>
                     </Button>
                     <Button variant="secondary" size="sm" className="justify-start px-4 h-11" onClick={() => notify.info('Diagnostics: Running integrity check...')}>
                       <span className="flex-1 text-left">Integrity Check</span>
                       <span className="text-[8px] opacity-40 font-mono uppercase tracking-widest">Verify DB Consistency</span>
                     </Button>
                     <Button variant="destructive" size="sm" className="justify-start px-4 h-11" onClick={() => openConfirm('Purge Database', 'This will IRREVERSIBLY destroy all history, collections, and variables. The app will restart after purge.', () => notify.error('Purge aborted: Action not yet implemented.'))}>
                       <span className="flex-1 text-left">Purge All Data</span>
                       <span className="text-[8px] opacity-60 font-mono uppercase tracking-widest text-rose-300">Wipe Data</span>
                     </Button>
                   </div>
                 </div>
               </div>
             </div>
          )}
        </div>
      )}
    >
      <PromptModal isOpen={promptConfig.isOpen} title={promptConfig.title} initialValue={promptConfig.initialValue} onClose={closePrompt} onSubmit={promptConfig.action} />
      <ConfirmModal isOpen={confirmConfig.isOpen} title={confirmConfig.title} message={confirmConfig.message} isDestructive={true} onClose={() => setConfirmConfig(prev => ({ ...prev, isOpen: false }))} onConfirm={confirmConfig.action} />
      <MultiGroupExportModal
        isOpen={exportModalOpen}
        groups={[...repeaterGroups, { id: 'null', name: 'Default (Uncategorized)' }]}
        onClose={() => setExportModalOpen(false)}
        onExport={handleExport}
      />
      {importData && (
        <ImportModal
          isOpen={isImportModalOpen}
          projectName={importData.name || 'Untitled Project'}
          environments={importData.all_environments || []}
          groups={importData.test_cases || []}
          onClose={() => setIsImportModalOpen(false)}
          onImport={(options) => {
            finalizeImport(importData, options, notify);
            setIsImportModalOpen(false);
          }}
        />
      )}
    </WorkspaceLayout>
  );
});
