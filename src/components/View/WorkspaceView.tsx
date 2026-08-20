import { useState, useImperativeHandle, forwardRef } from 'react';
import { useTraffic } from '@/hooks/traffic';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { ConfirmModal, PromptModal, MultiGroupExportModal, ImportModal } from '../Modals';
import { EnvironmentsSection } from '../modules/workspace/EnvironmentsSection';
import { CollectionsSection } from '../modules/workspace/CollectionsSection';
import { ReplacementsSection } from '../modules/workspace/ReplacementsSection';
import { Button, useDialog } from '../ui';
import { useNotification } from '../ui/NotificationProvider';

export type WorkspaceTab = 'env' | 'collections' | 'replacements';

export interface WorkspaceViewHandle {
  switchTab: (tab: WorkspaceTab) => void;
}

export const WorkspaceView = forwardRef<WorkspaceViewHandle, object>((_, ref) => {
  const { notify } = useNotification();
  const { alert } = useDialog();
  const {
    uiLayout, updateUILayout,
    variables, activeEnvId,
    environments,
    repeaterGroups, repeaterRequests,
    importPostman, importProject, finalizeImport, syncAll,
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
    all_variables?: Array<any>;
    test_cases?: Array<{ name: string; target: unknown[] }>;
    placeholders?: Record<string, any>;
    __isLegacy?: boolean;
  } | null>(null);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);

  const handleImportFileLoaded = (data: {
    name?: string;
    all_environments?: Array<{ id: string; name: string }>;
    all_variables?: Array<any>;
    placeholders?: Record<string, any>;
    __isLegacy?: boolean;
  }) => {
    // Detect legacy placeholders and present them as a virtual environment for selective import
    if ((!data.all_environments || data.all_environments.length === 0) && data.placeholders && Object.keys(data.placeholders).length > 0) {
      data.__isLegacy = true;
      data.all_environments = [{ id: 'legacy-env', name: 'Variables (Legacy Project Format)' }];
      
      // Convert placeholders to all_variables so the backend can import them correctly
      const all_variables: any[] = [];
      for (const [key, value] of Object.entries(data.placeholders)) {
        let values: any[] = [];
        if (Array.isArray(value)) {
          values = value.map((val, idx) => ({
            name: idx === 0 ? '(auto)' : `Value ${idx}`,
            value: String(val)
          }));
          if (!values.some(v => v.name === '(auto)')) {
            values.unshift({ name: '(auto)', value: String(value[0] || '') });
          }
        } else {
          values = [{
            name: '(auto)',
            value: String(value)
          }];
        }
        all_variables.push({
          environmentId: 'legacy-env',
          name: key,
          activeIndex: 0,
          values
        });
      }
      data.all_variables = all_variables;
    }

    // Ensure all variables in data.all_variables ALWAYS have an (auto) variant
    if (data.all_variables && Array.isArray(data.all_variables)) {
      data.all_variables.forEach((varObj: any) => {
        if (!Array.isArray(varObj.values) || varObj.values.length === 0) {
          varObj.values = [{ name: '(auto)', value: '' }];
        } else {
          const hasAuto = varObj.values.some((val: any) => val && val.name === '(auto)');
          if (!hasAuto) {
            const firstVal = varObj.values[0]?.value || '';
            varObj.values.unshift({ name: '(auto)', value: String(firstVal) });
          }
        }
      });
    }

    setImportData(data);
    setIsImportModalOpen(true);
  };

  const handleExport = async (selectedGroupIds: string[], projectName: string) => {
    try {
      const flattenedRequests = repeaterRequests.filter(r => selectedGroupIds.includes(r.groupId || 'null'));

      if (flattenedRequests.length === 0) {
        await alert('No requests', 'No requests found in selected groups.');
        return;
      }

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
            body_mode: req.bodyMode || undefined,
            body_json: req.bodyJson || undefined,
            body_urlencoded: req.bodyUrlencoded || undefined,
            body_multipart: req.bodyMultipart || undefined,
            url_params: req.urlParams || undefined,
            extract: req.extract || {},
            description: req.description || undefined
          };
        });

        const groupObj = repeaterGroups.find(g => g.id === gid);

        return {
          name: groupName,
          url: mostCommonBase,
          description: groupObj?.description || undefined,
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

    } catch (err) { await alert('Export failed', 'Export failed: ' + err); }
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
        </div>
      )}
      toolbarLeft={
        <div className="flex items-center px-4">
          <span className="text-[12px] font-black uppercase tracking-[0.2em] text-zinc-300">
            Workspace_Management / {activeTab === 'env' ? 'Environments' : activeTab === 'collections' ? 'Collections' : 'Replacements'}
          </span>
        </div>
      }

      toolbarRight={
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => importPostman(handleImportFileLoaded, notify)} className="text-zinc-500 hover:text-sky-text">Import PM</Button>
          <Button variant="ghost" size="sm" onClick={() => importProject(handleImportFileLoaded)} className="text-zinc-500 hover:text-sky-text">Import Project</Button>
          <Button variant="ghost" size="sm" onClick={() => setExportModalOpen(true)} className="text-zinc-500 hover:text-amber-400">Export</Button>
        </div>
      }
      mainContent={() => (
        <div className="w-full max-w-5xl mx-auto py-6 space-y-10">
          {activeTab === 'env' && <EnvironmentsSection openPrompt={openPrompt} openConfirm={openConfirm} />}
          {activeTab === 'collections' && <CollectionsSection selectedGroupId={selectedGroupId} setSelectedGroupId={setSelectedGroupId} openPrompt={openPrompt} openConfirm={openConfirm} />}
          {activeTab === 'replacements' && <ReplacementsSection />}
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
          onImport={async (options) => {
            await finalizeImport(importData, options, notify, syncAll);
            setIsImportModalOpen(false);
          }}
        />
      )}
    </WorkspaceLayout>
  );
});
