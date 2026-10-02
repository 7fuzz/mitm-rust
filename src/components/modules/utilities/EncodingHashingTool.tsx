import React, { useState } from 'react';
import { EncodingTool } from './EncodingTool';
import { HashingTool } from './HashingTool';

export const EncodingHashingTool: React.FC = () => {
  const [activeSection, setActiveSection] = useState<'encoding' | 'hashing'>('encoding');

  return (
    <div className="h-full flex flex-col gap-3">
      <div className="flex items-center gap-2 bg-surface border border-border rounded-lg p-1.5 shrink-0 text-xs">
        <button
          onClick={() => setActiveSection('encoding')}
          className={`px-3 py-1 rounded-md font-semibold transition-all ${
            activeSection === 'encoding'
              ? 'bg-primary text-primary-foreground shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          Encoding & Decoding
        </button>
        <button
          onClick={() => setActiveSection('hashing')}
          className={`px-3 py-1 rounded-md font-semibold transition-all ${
            activeSection === 'hashing'
              ? 'bg-primary text-primary-foreground shadow-xs'
              : 'text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
          }`}
        >
          Hashing & Bcrypt
        </button>
      </div>
      <div className="flex-1 min-h-0">
        {activeSection === 'encoding' ? <EncodingTool /> : <HashingTool />}
      </div>
    </div>
  );
};
