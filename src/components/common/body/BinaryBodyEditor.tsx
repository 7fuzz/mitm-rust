import React, { useMemo } from 'react';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { MingCuteIcon } from '../MingCuteIcon';
import { readFileAsBase64 } from '../../../services/tauri/bridge';
import { BINARY_PREFIX, binaryBodySize } from '../../../utils/bodyFormat';

interface BinaryBodyEditorProps {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
}

const formatSize = (b: number) => (b < 1024 ? `${b} B` : b < 1024 * 1024 ? `${(b / 1024).toFixed(1)} KB` : `${(b / 1024 / 1024).toFixed(2)} MB`);

export const BinaryBodyEditor: React.FC<BinaryBodyEditorProps> = ({ value, onChange, readOnly = false }) => {
  const size = useMemo(() => {
    try {
      return formatSize(binaryBodySize(value.startsWith(BINARY_PREFIX) ? value : BINARY_PREFIX + value));
    } catch {
      return 'invalid base64';
    }
  }, [value]);

  const loadFile = async () => {
    const path = await openFileDialog({ multiple: false, title: 'Load body from file' });
    if (typeof path !== 'string') return;
    const dataUrl = await readFileAsBase64(path);
    onChange(BINARY_PREFIX + dataUrl.slice(dataUrl.indexOf(',') + 1));
  };

  return (
    <div className="flex flex-col h-full min-h-0 gap-2 p-2">
      <div className="flex items-center gap-2 text-2xs text-muted-foreground font-sans">
        <span className="font-mono">{size}</span>
        {!readOnly && (
          <button
            type="button"
            onClick={loadFile}
            className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-foreground cursor-pointer text-xs"
          >
            <MingCuteIcon name="folder_open_line" size={13} />
            <span>Load file</span>
          </button>
        )}
      </div>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        readOnly={readOnly}
        spellCheck={false}
        className="flex-1 min-h-[120px] w-full bg-background border border-border rounded p-2 font-mono text-2xs text-muted-foreground break-all focus:outline-none focus:border-primary resize-none"
      />
    </div>
  );
};
