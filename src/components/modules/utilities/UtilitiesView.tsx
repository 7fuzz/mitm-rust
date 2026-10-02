import React, { useState } from 'react';
import { CvssCalculator } from './CvssCalculator';
import { JsonToolkit } from './JsonToolkit';
import { EncodingTool } from './EncodingTool';
import { HashingTool } from './HashingTool';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const UtilitiesView: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'cvss' | 'json' | 'encoding' | 'hashing'>('cvss');

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
          <MingCuteIcon name="transfer_line" size={15} />
          <span>Encoding & Decoding</span>
        </button>

        <button
          onClick={() => setActiveTab('hashing')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-md font-medium transition-all ${
            activeTab === 'hashing'
              ? 'bg-primary text-primary-foreground font-bold shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          <MingCuteIcon name="hash_line" size={15} />
          <span>Hashing & Bcrypt</span>
        </button>
      </div>

      {/* Main Tool Content Container */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {activeTab === 'cvss' && (
          <div className="h-full overflow-y-auto">
            <CvssCalculator />
          </div>
        )}
        {activeTab === 'json' && <JsonToolkit />}
        {activeTab === 'encoding' && <EncodingTool />}
        {activeTab === 'hashing' && <HashingTool />}
      </div>
    </div>
  );
};
