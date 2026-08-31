import React, { useState } from 'react';
import { CvssCalculator } from './CvssCalculator';
import { JsonToolkit } from './JsonToolkit';
import { EncodingHashingTool } from './EncodingHashingTool';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const UtilitiesView: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'cvss' | 'json' | 'encoding'>('cvss');

  return (
    <div className="h-full flex flex-col bg-background p-4 overflow-hidden">
      {/* Top Utility Tool Tabs */}
      <div className="bg-surface border border-border rounded-lg p-2 flex items-center gap-2 mb-4 shrink-0 shadow-2xs text-xs select-none">
        <button
          onClick={() => setActiveTab('cvss')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-md font-medium transition-all ${
            activeTab === 'cvss'
              ? 'bg-primary text-primary-foreground font-bold shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          <MingCuteIcon name="calculator_line" size={15} />
          <span>CVSS Score Calculator</span>
        </button>

        <button
          onClick={() => setActiveTab('json')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-md font-medium transition-all ${
            activeTab === 'json'
              ? 'bg-primary text-primary-foreground font-bold shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          <MingCuteIcon name="code_line" size={15} />
          <span>JSON Toolkit</span>
        </button>

        <button
          onClick={() => setActiveTab('encoding')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-md font-medium transition-all ${
            activeTab === 'encoding'
              ? 'bg-primary text-primary-foreground font-bold shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          <MingCuteIcon name="hash_line" size={15} />
          <span>Encoding & Hashing Tool</span>
        </button>
      </div>

      {/* Main Tool Content Container */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'cvss' && <CvssCalculator />}
        {activeTab === 'json' && <JsonToolkit />}
        {activeTab === 'encoding' && <EncodingHashingTool />}
      </div>
    </div>
  );
};
