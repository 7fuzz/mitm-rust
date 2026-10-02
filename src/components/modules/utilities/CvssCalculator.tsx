import React, { useState } from 'react';
import { MingCuteIcon } from '../../common/MingCuteIcon';

interface CvssOption {
  key: string;
  name: string;
  options: { label: string; val: string; weight: number }[];
}

const CVSS_METRICS: CvssOption[] = [
  {
    key: 'AV',
    name: 'Attack Vector (AV)',
    options: [
      { label: 'Network (N)', val: 'N', weight: 0.85 },
      { label: 'Adjacent (A)', val: 'A', weight: 0.62 },
      { label: 'Local (L)', val: 'L', weight: 0.55 },
      { label: 'Physical (P)', val: 'P', weight: 0.2 },
    ],
  },
  {
    key: 'AC',
    name: 'Attack Complexity (AC)',
    options: [
      { label: 'Low (L)', val: 'L', weight: 0.77 },
      { label: 'High (H)', val: 'H', weight: 0.44 },
    ],
  },
  {
    key: 'PR',
    name: 'Privileges Required (PR)',
    options: [
      { label: 'None (N)', val: 'N', weight: 0.85 },
      { label: 'Low (L)', val: 'L', weight: 0.62 },
      { label: 'High (H)', val: 'H', weight: 0.27 },
    ],
  },
  {
    key: 'UI',
    name: 'User Interaction (UI)',
    options: [
      { label: 'None (N)', val: 'N', weight: 0.85 },
      { label: 'Required (R)', val: 'R', weight: 0.62 },
    ],
  },
  {
    key: 'S',
    name: 'Scope (S)',
    options: [
      { label: 'Unchanged (U)', val: 'U', weight: 1.0 },
      { label: 'Changed (C)', val: 'C', weight: 1.08 },
    ],
  },
  {
    key: 'C',
    name: 'Confidentiality (C)',
    options: [
      { label: 'High (H)', val: 'H', weight: 0.56 },
      { label: 'Low (L)', val: 'L', weight: 0.22 },
      { label: 'None (N)', val: 'N', weight: 0.0 },
    ],
  },
  {
    key: 'I',
    name: 'Integrity (I)',
    options: [
      { label: 'High (H)', val: 'H', weight: 0.56 },
      { label: 'Low (L)', val: 'L', weight: 0.22 },
      { label: 'None (N)', val: 'N', weight: 0.0 },
    ],
  },
  {
    key: 'A',
    name: 'Availability (A)',
    options: [
      { label: 'High (H)', val: 'H', weight: 0.56 },
      { label: 'Low (L)', val: 'L', weight: 0.22 },
      { label: 'None (N)', val: 'N', weight: 0.0 },
    ],
  },
];

export const CvssCalculator: React.FC = () => {
  const [selections, setSelections] = useState<Record<string, string>>({
    AV: 'N',
    AC: 'L',
    PR: 'N',
    UI: 'N',
    S: 'U',
    C: 'H',
    I: 'H',
    A: 'H',
  });

  const handleSelect = (key: string, val: string) => {
    setSelections((prev) => ({ ...prev, [key]: val }));
  };

  // Compute vector string
  const vectorString = `CVSS:3.1/AV:${selections.AV}/AC:${selections.AC}/PR:${selections.PR}/UI:${selections.UI}/S:${selections.S}/C:${selections.C}/I:${selections.I}/A:${selections.A}`;

  // Estimate score algorithm
  const computeScore = () => {
    const av = CVSS_METRICS[0].options.find((o) => o.val === selections.AV)?.weight || 0.85;
    const ac = CVSS_METRICS[1].options.find((o) => o.val === selections.AC)?.weight || 0.77;
    const pr = CVSS_METRICS[2].options.find((o) => o.val === selections.PR)?.weight || 0.85;
    const ui = CVSS_METRICS[3].options.find((o) => o.val === selections.UI)?.weight || 0.85;
    const c = CVSS_METRICS[5].options.find((o) => o.val === selections.C)?.weight || 0.56;
    const i = CVSS_METRICS[6].options.find((o) => o.val === selections.I)?.weight || 0.56;
    const a = CVSS_METRICS[7].options.find((o) => o.val === selections.A)?.weight || 0.56;

    const iss = 1 - (1 - c) * (1 - i) * (1 - a);
    const impact = selections.S === 'U' ? 6.42 * iss : 7.52 * (iss - 0.029) - 3.25 * Math.pow(iss - 0.02, 15);
    const exploitability = 8.22 * av * ac * pr * ui;

    if (impact <= 0) return 0.0;
    const rawScore = selections.S === 'U' ? Math.min(impact + exploitability, 10) : Math.min(1.08 * (impact + exploitability), 10);
    return Math.ceil(rawScore * 10) / 10;
  };

  const score = computeScore();

  let severity = 'LOW';
  let badgeColor = 'bg-sky-500 text-white';
  if (score >= 9.0) {
    severity = 'CRITICAL';
    badgeColor = 'bg-rose-600 text-white';
  } else if (score >= 7.0) {
    severity = 'HIGH';
    badgeColor = 'bg-rose-500 text-white';
  } else if (score >= 4.0) {
    severity = 'MEDIUM';
    badgeColor = 'bg-amber-500 text-white';
  }

  return (
    <div className="space-y-4 text-xs">
      {/* Score Banner */}
      <div className="p-4 bg-surface border border-border rounded-lg flex items-center justify-between shadow-2xs">
        <div>
          <span className="text-muted-foreground text-xs uppercase font-medium">CVSS v3.1 Base Score</span>
          <div className="flex items-center gap-3 mt-1 font-mono">
            <span className="text-3xl font-extrabold text-foreground">{score.toFixed(1)}</span>
            <span className={`px-3 py-1 rounded-full text-xs font-extrabold uppercase ${badgeColor}`}>
              {severity}
            </span>
          </div>
        </div>

        <div className="flex-1 max-w-lg ml-4">
          <span className="text-2xs text-muted-foreground block mb-1">Vector String:</span>
          <div className="flex items-center gap-2">
            <code className="flex-1 bg-background border border-border rounded px-2.5 py-1 text-xs text-primary font-mono select-all">
              {vectorString}
            </code>
            <button
              onClick={() => navigator.clipboard.writeText(vectorString)}
              className="px-2.5 py-1 bg-neutral-subtle border border-border rounded text-foreground hover:bg-surface font-medium flex items-center gap-1"
            >
              <MingCuteIcon name="copy_line" size={13} />
              <span>Copy</span>
            </button>
          </div>
        </div>
      </div>

      {/* Radio Matrix */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
        {CVSS_METRICS.map((metric) => (
          <div key={metric.key} className="bg-surface border border-border rounded-lg p-3 space-y-2">
            <span className="font-semibold text-foreground text-xs block">{metric.name}</span>
            <div className="space-y-1">
              {metric.options.map((opt) => {
                const isChecked = selections[metric.key] === opt.val;
                return (
                  <button
                    key={opt.val}
                    type="button"
                    onClick={() => handleSelect(metric.key, opt.val)}
                    className={`w-full px-2.5 py-1.5 rounded text-left font-mono text-xs flex items-center justify-between border transition-all ${
                      isChecked
                        ? 'bg-primary/15 border-primary font-bold text-foreground'
                        : 'border-border/60 bg-background/50 hover:border-border text-muted-foreground'
                    }`}
                  >
                    <span>{opt.label}</span>
                    {isChecked && <MingCuteIcon name="check_line" size={14} className="text-primary" />}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
