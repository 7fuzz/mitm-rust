import { useState } from 'react';
import { WorkspaceLayout } from '../Layout/WorkspaceLayout';
import { useTraffic } from '@/hooks/traffic';

// Import our standalone tools
import { JsonTool, CvssTool } from '../Tools';

// The Tool Registry
const TOOLS = [
  { id: 'json', name: 'JSON Toolkit' },
  { id: 'cvss', name: 'CVSS Calculator' },
];

export function UtilitiesView() {
  const { uiLayout, updateUILayout } = useTraffic();
  const [activeToolId, setActiveToolId] = useState('json');

  return (
    <WorkspaceLayout
      uiLayout={uiLayout}
      onUpdateLayout={updateUILayout}
      listComponent={() => (
        <div className="flex flex-col p-2 gap-1 h-full">
          <div className="px-3 py-2 text-[9px] font-black tracking-widest text-zinc-600 uppercase mb-2">Available Tools</div>
          {TOOLS.map(tool => (
            <button
              key={tool.id}
              onClick={() => setActiveToolId(tool.id)}
              className={`flex items-center text-left px-3 py-2.5 rounded text-[11px] font-bold tracking-wider transition-all border ${activeToolId === tool.id ? 'bg-fuchsia-500/10 border-fuchsia-500/30 text-fuchsia-400' : 'bg-transparent border-transparent text-zinc-400 hover:bg-zinc-900'}`}
            >
              {tool.name}
            </button>
          ))}
        </div>
      )}
      toolbarLeft={
        <div className="flex items-center px-4">
          <span className="text-[12px] font-black uppercase tracking-[0.2em] text-zinc-300">
            {TOOLS.find(t => t.id === activeToolId)?.name}
          </span>
        </div>
      }
      mainContent={(splitMode) => (
        <div className="w-full h-full">
          {activeToolId === 'json' && <JsonTool splitMode={splitMode} />}
          {activeToolId === 'cvss' && <CvssTool splitMode={splitMode} />}
        </div>
      )}
    />
  );
}
