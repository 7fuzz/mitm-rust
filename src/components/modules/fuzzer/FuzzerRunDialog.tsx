import React, { useEffect, useState } from 'react';
import { Button, Dialog, Input } from '../../common/ui';
import type { AttackType } from '../../../services/tauri/bridge';

interface FuzzerRunDialogProps {
  mode: 'start' | 'save' | null;
  estimate: number | null;
  variableCount: number;
  attackType: AttackType;
  onClose: () => void;
  onStart: (name: string, save: boolean) => void;
  onSave: (name: string) => void;
}

const ATTACK_LABEL: Record<AttackType, string> = {
  sniper: 'Sniper',
  pitchfork: 'Pitchfork',
  clusterbomb: 'Cluster bomb',
};

const timestampName = () => {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
};

const Stat: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="flex-1 min-w-0 rounded-lg bg-background border border-border px-3 py-2">
    <div className="text-sm font-semibold text-foreground font-mono tabular-nums truncate">{value}</div>
    <div className="text-3xs text-muted-foreground uppercase tracking-wider">{label}</div>
  </div>
);

export const FuzzerRunDialog: React.FC<FuzzerRunDialogProps> = ({
  mode,
  estimate,
  variableCount,
  attackType,
  onClose,
  onStart,
  onSave,
}) => {
  const [defaultName, setDefaultName] = useState('');
  const [name, setName] = useState('');

  useEffect(() => {
    if (!mode) return;
    const stamp = timestampName();
    setDefaultName(stamp);
    setName(stamp);
  }, [mode]);

  const submit = (save: boolean) => {
    const finalName = name.trim() || defaultName;
    if (mode === 'save') onSave(finalName);
    else onStart(finalName, save);
    onClose();
  };

  return (
    <Dialog
      isOpen={mode !== null}
      onClose={onClose}
      size="sm"
      title={mode === 'save' ? 'Save run' : 'Start run'}
      footer={
        mode === 'save' ? (
          <>
            <Button variant="secondary" onClick={onClose}>Cancel</Button>
            <Button variant="primary" icon="download_line" onClick={() => submit(true)}>Save</Button>
          </>
        ) : (
          <>
            <Button variant="secondary" icon="play_line" onClick={() => submit(false)}>Run temporary</Button>
            <Button variant="primary" icon="download_line" onClick={() => submit(true)}>Save &amp; run</Button>
          </>
        )
      }
    >
      <div className="flex gap-2">
        <Stat label="Requests" value={estimate !== null ? estimate.toLocaleString() : '—'} />
        <Stat label={variableCount === 1 ? 'Variable' : 'Variables'} value={String(variableCount)} />
        <Stat label="Attack" value={ATTACK_LABEL[attackType]} />
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          submit(true);
        }}
      >
        <label className="block text-3xs text-muted-foreground uppercase tracking-wider mb-1">Run name</label>
        <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder={defaultName} onFocus={(e) => e.target.select()} sizeVariant="sm" />
      </form>
    </Dialog>
  );
};
