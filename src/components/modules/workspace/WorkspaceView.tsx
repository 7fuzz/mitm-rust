import React, { useEffect } from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { WorkspaceSelector } from './WorkspaceSelector';
import { ProjectImportModal } from './ProjectImportModal';
import { WorkspaceSettingsPanel } from './WorkspaceSettingsPanel';
import { VariablesManager } from './VariablesManager';
import { ReplacementRules } from './ReplacementRules';
import { CollectionLinking } from './CollectionLinking';

export const WorkspaceView: React.FC = () => {
  const { initStore } = useWorkspaceStore();

  useEffect(() => {
    initStore();
  }, [initStore]);

  return (
    <div className="h-full flex flex-col bg-background overflow-hidden">
      {/* Top Header Workspace Switcher Bar */}
      <div className="bg-header border-b border-border p-2.5 flex items-center justify-between shrink-0">
        <WorkspaceSelector />
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-5">
        <WorkspaceSettingsPanel />
        <VariablesManager />
        <ReplacementRules />
        <CollectionLinking />
      </div>

      {/* Project Import Modal */}
      <ProjectImportModal />
    </div>
  );
};
