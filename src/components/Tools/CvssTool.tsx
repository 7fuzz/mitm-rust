import { useState, useMemo } from 'react';
import { useNotification } from '../ui/NotificationProvider';
import { CVSS40 } from '@pandatix/js-cvss';

const CVSS4_GROUPS = [
  {
    title: 'Exploitability Metrics',
    metrics: [
      { id: 'AV', name: 'Attack Vector', opts: { N: 'Network', A: 'Adjacent', L: 'Local', P: 'Physical' } },
      { id: 'AC', name: 'Attack Complexity', opts: { L: 'Low', H: 'High' } },
      { id: 'AT', name: 'Attack Requirements', opts: { N: 'None', P: 'Present' } },
      { id: 'PR', name: 'Privileges Required', opts: { N: 'None', L: 'Low', H: 'High' } },
      { id: 'UI', name: 'User Interaction', opts: { N: 'None', P: 'Passive', A: 'Active' } },
    ]
  },
  {
    title: 'Vulnerable System Impact',
    metrics: [
      { id: 'VC', name: 'Confidentiality', opts: { H: 'High', L: 'Low', N: 'None' } },
      { id: 'VI', name: 'Integrity', opts: { H: 'High', L: 'Low', N: 'None' } },
      { id: 'VA', name: 'Availability', opts: { H: 'High', L: 'Low', N: 'None' } },
    ]
  },
  {
    title: 'Subsequent System Impact',
    metrics: [
      { id: 'SC', name: 'Confidentiality', opts: { H: 'High', L: 'Low', N: 'None' } },
      { id: 'SI', name: 'Integrity', opts: { H: 'High', L: 'Low', N: 'None' } },
      { id: 'SA', name: 'Availability', opts: { H: 'High', L: 'Low', N: 'None' } },
    ]
  }
];

function MetricGroupPanel({ group, vector, updateMetric }: { group: typeof CVSS4_GROUPS[0], vector: Record<string, string>, updateMetric: (id: string, val: string) => void }) {
  return (
    <div className="flex flex-col gap-6 bg-zinc-900/20 p-6 xl:p-8 rounded-xl border border-zinc-800/50 shadow-sm">
      <h3 className="text-[11px] font-black uppercase tracking-widest text-zinc-400 border-b border-zinc-800 pb-3">
        {group.title}
      </h3>
      {group.metrics.map(metric => (
        <div key={metric.id} className="flex flex-col gap-1">
          <span className="text-[9px] text-zinc-500 font-bold uppercase tracking-widest flex items-center gap-2">
            <span className="opacity-30">#</span> {metric.name} ({metric.id})
          </span>
          <div className="flex bg-zinc-950 p-1.5 gap-1 rounded-lg border border-zinc-800 w-full overflow-hidden">
            {Object.entries(metric.opts).map(([k, v]) => {
              const isActive = vector[metric.id] === k;
              const activeClass = isActive
                ? 'bg-fuchsia-500/20 border-fuchsia-500 text-fuchsia-300 ring-1 ring-fuchsia-500/50'
                : 'bg-zinc-900 border-transparent text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300';

              return (
                <button
                  key={k}
                  onClick={() => updateMetric(metric.id, k)}
                  className={`flex-1 text-[9px] uppercase font-bold tracking-widest px-1 py-1.5 rounded border transition-all ${activeClass}`}
                >
                  {k} - {v}
                </button>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

export function CvssTool({ splitMode }: { splitMode: 'horizontal' | 'vertical' }) {
  const { notify } = useNotification();

  const [vector, setVector] = useState<Record<string, string>>({
    AV: 'N', AC: 'L', AT: 'N', PR: 'N', UI: 'N', VC: 'N', VI: 'N', VA: 'N', SC: 'N', SI: 'N', SA: 'N'
  });

  const [inputString, setInputString] = useState(`CVSS:4.0/AV:${vector.AV}/AC:${vector.AC}/AT:${vector.AT}/PR:${vector.PR}/UI:${vector.UI}/VC:${vector.VC}/VI:${vector.VI}/VA:${vector.VA}/SC:${vector.SC}/SI:${vector.SI}/SA:${vector.SA}`);
  const [prevVector, setPrevVector] = useState(vector);

  if (vector !== prevVector) {
    setPrevVector(vector);
    const cv = `CVSS:4.0/AV:${vector.AV}/AC:${vector.AC}/AT:${vector.AT}/PR:${vector.PR}/UI:${vector.UI}/VC:${vector.VC}/VI:${vector.VI}/VA:${vector.VA}/SC:${vector.SC}/SI:${vector.SI}/SA:${vector.SA}`;
    setInputString(cv);
  }

  const score = useMemo(() => {
    const currentVector = `CVSS:4.0/AV:${vector.AV}/AC:${vector.AC}/AT:${vector.AT}/PR:${vector.PR}/UI:${vector.UI}/VC:${vector.VC}/VI:${vector.VI}/VA:${vector.VA}/SC:${vector.SC}/SI:${vector.SI}/SA:${vector.SA}`;
    try {
      const vecObj = new CVSS40(currentVector);
      const scoreValue = vecObj.Score();

      let severity = 'None';
      if (scoreValue >= 0.1 && scoreValue <= 3.9) severity = 'Low';
      else if (scoreValue >= 4.0 && scoreValue <= 6.9) severity = 'Medium';
      else if (scoreValue >= 7.0 && scoreValue <= 8.9) severity = 'High';
      else if (scoreValue >= 9.0) severity = 'Critical';

      return { value: scoreValue, severity };
    } catch (_e) {
      return { value: 0.0, severity: 'Invalid' };
    }
  }, [vector]);

  const updateMetric = (metric: string, val: string) => {
    setVector(prev => ({ ...prev, [metric]: val }));
  };

  const handleInputParse = (val: string) => {
    setInputString(val);
    const parts = val.toUpperCase().split('/');
    const newVec = { ...vector };
    let hasChanges = false;

    parts.forEach(p => {
      const [k, v] = p.split(':');
      if (newVec.hasOwnProperty(k) && Object.keys(CVSS4_GROUPS.flatMap(g => g.metrics).find(m => m.id === k)?.opts || {}).includes(v)) {
        newVec[k] = v;
        hasChanges = true;
      }
    });

    if (hasChanges) setVector(newVec);
  };

  const copyForSheets = () => {
    const sheetData = `${inputString}\t${vector.AV}\t${vector.AC}\t${vector.AT}\t${vector.PR}\t${vector.UI}\t${vector.VC}\t${vector.VI}\t${vector.VA}\t${vector.SC}\t${vector.SI}\t${vector.SA}\t${score.value}\t${score.severity}`;
    navigator.clipboard.writeText(sheetData);
    notify.success('Copied Vector & Score for Processing');
  };

  const openOfficialCalculator = () => {
    window.open(`https://www.first.org/cvss/calculator/4.0#${inputString}`, '_blank');
  };

  const handleReset = () => {
    setVector({
      AV: 'N', AC: 'L', AT: 'N', PR: 'N', UI: 'N', VC: 'N', VI: 'N', VA: 'N', SC: 'N', SI: 'N', SA: 'N'
    });
    notify.info('Calculator Reset');
  };

  const sevColor = score.severity === 'Critical' ? 'bg-rose-500/20 text-rose-400 border-rose-500/50' :
    score.severity === 'High' ? 'bg-orange-500/20 text-orange-400 border-orange-500/50' :
      score.severity === 'Medium' ? 'bg-amber-500/20 text-amber-400 border-amber-500/50' :
        score.severity === 'Low' ? 'bg-sky-500/20 text-sky-text border-sky-500/50' :
          'bg-zinc-800 text-zinc-500 border-zinc-700';

  const exploitabilityGroup = CVSS4_GROUPS[0];
  const impactGroups = [CVSS4_GROUPS[1], CVSS4_GROUPS[2]];

  return (
    <div className={`space-y-8 mx-auto ${splitMode === 'horizontal' ? 'max-w-360' : 'max-w-4xl'}`}>

      {/* Top Console */}
      <div className="p-4 bg-zinc-900/50 border border-zinc-800 rounded flex flex-col xl:flex-row xl:items-center gap-4 justify-between shadow-inner shadow-app-shadow/20">

        <div className="flex flex-col gap-1.5 flex-1 w-full">
          <span className="text-zinc-500 text-[10px] font-bold uppercase tracking-widest">CVSS 4.0 Vector (Editable)</span>
          <input
            value={inputString}
            onChange={(e) => handleInputParse(e.target.value)}
            className="w-full bg-zinc-950 border border-zinc-700 p-2 rounded text-fuchsia-400 text-xs font-black font-mono outline-none focus:border-fuchsia-500 transition-colors"
            placeholder="Paste CVSS:4.0/... vector here"
            spellCheck={false}
          />
        </div>

        <div className="flex items-center gap-4 shrink-0 flex-wrap">
          <div className="flex flex-col items-center">
            <span className="text-zinc-500 text-[9px] font-bold uppercase tracking-widest mb-1">Base Score</span>
            <div className={`px-4 py-1.5 rounded border text-sm font-black tracking-widest uppercase flex items-center gap-2 ${sevColor}`}>
              <span>{score.value.toFixed(1)}</span>
              <span className="opacity-50">|</span>
              <span className="text-[10px]">{score.severity}</span>
            </div>
          </div>

          <div className="w-px h-10 bg-zinc-800 hidden md:block"></div>

          <div className="flex gap-2 w-full md:w-auto mt-2 md:mt-0">
            <button
              onClick={handleReset}
              className="flex-1 md:flex-none px-4 py-2 bg-zinc-900 hover:bg-rose-900/30 text-zinc-400 hover:text-rose-400 text-[10px] rounded border border-zinc-700 hover:border-rose-800 transition-all uppercase font-bold"
              title="Reset Calculator"
            >
              Reset
            </button>
            <button
              onClick={copyForSheets}
              className="flex-1 md:flex-none px-4 py-2 bg-fuchsia-900/30 hover:bg-fuchsia-600 text-fuchsia-400 hover:text-zinc-950 text-[10px] rounded border border-fuchsia-800 transition-all uppercase font-bold"
            >
              Copy Data
            </button>
            <button
              onClick={openOfficialCalculator}
              className="flex-1 md:flex-none px-4 py-2 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 text-[10px] rounded border border-zinc-700 transition-all uppercase font-bold flex items-center justify-center gap-2"
              title="Open Official Calculator"
            >
              Official
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="7" y1="17" x2="17" y2="7"></line><polyline points="7 7 17 7 17 17"></polyline></svg>
            </button>
          </div>
        </div>
      </div>

      <div className={`grid gap-8 items-start ${splitMode === 'horizontal' ? 'grid-cols-2' : 'grid-cols-1'}`}>
        <div className="flex flex-col gap-8">
          <MetricGroupPanel group={exploitabilityGroup} vector={vector} updateMetric={updateMetric} />
        </div>

        <div className="flex flex-col gap-8">
          {impactGroups.map((group, idx) => (
            <MetricGroupPanel key={idx} group={group} vector={vector} updateMetric={updateMetric} />
          ))}
        </div>

      </div>
    </div>
  );
}
