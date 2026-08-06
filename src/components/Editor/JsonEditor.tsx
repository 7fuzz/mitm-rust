import { useState, useMemo, useCallback } from 'react';
import { Select } from '../ui';

// --- Sub-component: Handles typing without losing focus ---
const EditableKey = ({ initialKey, onCommit }: { initialKey: string, onCommit: (oldK: string, newK: string) => void }) => {
  const [localKey, setLocalKey] = useState(initialKey);
  const [prevInitialKey, setPrevInitialKey] = useState(initialKey);

  if (initialKey !== prevInitialKey) {
    setPrevInitialKey(initialKey);
    setLocalKey(initialKey);
  }

  const handleBlur = () => {
    const trimmed = localKey.trim();
    if (trimmed !== initialKey && trimmed !== '') {
      onCommit(initialKey, trimmed);
    } else {
      setLocalKey(initialKey);
    }
  };

  return (
    <div className="w-1/3 flex items-center shrink-0 group/key">
      <span className="text-zinc-600 font-mono text-[11px] mr-1">&quot;</span>
      <input
        value={localKey}
        onChange={(e) => setLocalKey(e.target.value)}
        onBlur={handleBlur}
        className="flex-1 bg-transparent border-b border-transparent hover:border-zinc-700 focus:border-sky-500 text-sky-text text-[11px] font-mono outline-none min-w-0 transition-colors"
      />
      <span className="text-zinc-600 font-mono text-[11px] ml-1">&quot;:</span>
    </div>
  );
};

// --- Recursive Node Component ---
interface JsonNodeProps {
  label: string | null;
  value: unknown;
  onChange: (newVal: unknown) => void;
  onDelete?: () => void;
  onKeyChange?: (oldKey: string, newKey: string) => void;
}

const JsonNode = ({ label, value, onChange, onDelete, onKeyChange }: JsonNodeProps) => {
  const isArray = Array.isArray(value);
  const isObject = value !== null && typeof value === 'object' && !isArray;
  const [isFileType, setIsFileType] = useState(() => {
    return typeof value === 'string' && (value.startsWith('data:') || value.startsWith('[FILE]'));
  });

  if (isObject || isArray) {
    const keys = Object.keys(value as object);

    const handleChildChange = (key: string, newVal: unknown) => {
      const cloned = isArray ? [...(value as unknown[])] : { ...(value as Record<string, unknown>) };
      if (isArray) (cloned as unknown[])[Number(key)] = newVal;
      else (cloned as Record<string, unknown>)[key] = newVal;
      onChange(cloned);
    };

    const handleChildDelete = (key: string) => {
      if (isArray) {
        const cloned = [...(value as unknown[])];
        cloned.splice(Number(key), 1);
        onChange(cloned);
      } else {
        const cloned = { ...(value as Record<string, unknown>) };
        delete cloned[key];
        onChange(cloned);
      }
    };

    const handleChildKeyChange = (oldKey: string, newKey: string) => {
      if (oldKey === newKey || isArray) return;
      const original = value as Record<string, unknown>;
      const newObj: Record<string, unknown> = {};
      for (const k in original) {
        if (k === oldKey) newObj[newKey] = original[oldKey];
        else newObj[k] = original[k];
      }
      onChange(newObj);
    };

    const handleAdd = () => {
      if (isArray) {
        onChange([...(value as unknown[]), ""]);
      } else {
        const next = { ...(value as Record<string, unknown>), [`new_key_${Date.now().toString().slice(-4)}`]: "" };
        onChange(next);
      }
    };

    return (
      <div className="ml-4 pl-3 border-l border-zinc-800/80 space-y-2 mt-2">
        <div className="flex justify-between items-center group/node">
          <div className="flex items-center flex-1">
            {label !== null && onKeyChange && (
              <EditableKey initialKey={label} onCommit={onKeyChange || (() => {})} />
            )}
            <span className="text-[10px] text-zinc-500 font-bold uppercase ml-2">
              {isArray ? `Array [${keys.length}]` : `Object {${keys.length}}`}
            </span>
          </div>
          {onDelete && (
            <button onClick={onDelete} className="text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 px-1.5 py-0.5 rounded opacity-0 group-hover/node:opacity-100 transition-all text-xs">
              ✕
            </button>
          )}
        </div>

        {keys.map((k) => (
          <div key={k} className="mt-1">
            <JsonNode
              label={isArray ? null : k}
              value={(value as Record<string, unknown>)[k]}
              onChange={(newVal: unknown) => handleChildChange(k, newVal)}
              onDelete={() => handleChildDelete(k)}
              onKeyChange={isArray ? undefined : handleChildKeyChange}
            />
          </div>
        ))}
        <button onClick={handleAdd} className="text-[9px] text-sky-text hover:text-sky-text mt-2 font-bold uppercase tracking-widest px-1 hover:bg-sky-500/10 rounded transition-colors">
          + Add {isArray ? 'Item' : 'Key'}
        </button>
      </div>
    );
  }

  const rawType = value === null ? 'null' : typeof value;
  const valueType = isFileType ? 'file' : rawType;

  const handleTypeSwitch = (newType: string) => {
    if (newType === 'file') {
      setIsFileType(true);
      if (typeof value !== 'string') onChange('');
    } else {
      setIsFileType(false);
      if (newType === 'string') onChange(String(value ?? ''));
      else if (newType === 'number') onChange(Number(value) || 0);
      else if (newType === 'boolean') onChange(Boolean(value));
      else if (newType === 'null') onChange(null);
    }
  };

  const handleFileAttach = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        onChange(reader.result);
      }
    };
    reader.readAsDataURL(file);
    e.target.value = '';
  };

  return (
    <div className="flex gap-2 items-center group/leaf">
      {label !== null ? (
        <EditableKey initialKey={label} onCommit={onKeyChange || (() => {})} />
      ) : (
        <div className="w-4 shrink-0 text-zinc-600 text-[10px] flex justify-end pr-2">-</div>
      )}

      <Select
        value={valueType}
        onChange={(val) => handleTypeSwitch(val)}
        options={[
          { value: "string", label: "STR" },
          { value: "number", label: "NUM" },
          { value: "boolean", label: "BOOL" },
          { value: "file", label: "FILE" },
          { value: "null", label: "NULL" }
        ]}
        className="w-20"
      />

      {valueType === 'boolean' ? (
        <Select
          value={String(value)}
          onChange={(val) => onChange(val === 'true')}
          options={[
            { value: "true", label: "true" },
            { value: "false", label: "false" }
          ]}
          className="flex-1"
        />
      ) : valueType === 'null' ? (
        <div className="flex-1 bg-zinc-950/30 border border-transparent p-1.5 rounded text-zinc-600 text-[11px] font-mono italic">
          null
        </div>
      ) : valueType === 'file' ? (
        <div className="flex-1 flex items-center gap-2 bg-zinc-950/50 border border-zinc-800/50 hover:border-zinc-700 focus-within:border-sky-500 rounded p-1 transition-colors overflow-hidden min-w-0">
          <input
            type="file"
            onChange={handleFileAttach}
            className="text-[10px] text-zinc-400 file:mr-2 file:py-0.5 file:px-2 file:rounded file:border-0 file:text-[9px] file:font-semibold file:bg-zinc-800 file:text-zinc-300 hover:file:bg-zinc-700 cursor-pointer flex-1 min-w-0"
          />
          {typeof value === 'string' && value.startsWith('data:') && (
            <span className="text-[9px] text-purple-400 font-mono truncate max-w-28 shrink-0 bg-purple-500/10 px-1.5 py-0.5 rounded border border-purple-500/20" title={value}>
              Base64 ({Math.round((value.length * 0.75) / 1024)} KB)
            </span>
          )}
        </div>
      ) : (
        <div className="flex-1 flex items-center bg-zinc-950/50 border border-zinc-800/50 hover:border-zinc-700 focus-within:border-sky-500 rounded transition-colors overflow-hidden">
          {valueType === 'string' && <span className="text-zinc-600 pl-2">&quot;</span>}
          <input
            type={valueType === 'number' ? 'number' : 'text'}
            value={value as string | number}
            onChange={(e) => onChange(valueType === 'number' ? Number(e.target.value) : e.target.value)}
            className={`w-full bg-transparent p-1.5 outline-none text-[11px] font-mono ${valueType === 'number' ? 'text-sky-text' : 'text-emerald-text'}`}
          />
          {valueType === 'string' && <span className="text-zinc-600 pr-2">&quot;</span>}
        </div>
      )}

      {onDelete && (
        <button onClick={onDelete} className="p-1.5 text-zinc-600 hover:text-rose-500 hover:bg-rose-500/10 rounded opacity-0 group-hover/leaf:opacity-100 transition-all">
          ✕
        </button>
      )}
    </div>
  );
};

export function JsonEditor({ initialBody, onChange }: { initialBody: string, onChange: (v: string) => void }) {
  const { parsed, error } = useMemo(() => {
    try {
      if (!initialBody || initialBody.trim() === '') {
        return { parsed: {}, error: '' };
      }
      return { parsed: JSON.parse(initialBody), error: '' };
    } catch (_e) {
      return { parsed: null, error: 'Invalid JSON syntax. Fix it in Raw mode first.' };
    }
  }, [initialBody]);

  const handleChange = useCallback((newObj: unknown) => {
    onChange(JSON.stringify(newObj, null, 2));
  }, [onChange]);

  if (error) return (
    <div className="text-rose-400 text-xs p-4 border border-rose-500/30 rounded bg-rose-500/10 font-mono m-4">
      {error}
    </div>
  );
  if (parsed === null) return null;

  return (
    <div className="p-4 bg-zinc-900/30 rounded overflow-x-auto min-h-full">
      <JsonNode
        label={null}
        value={parsed}
        onChange={handleChange}
      />
    </div>
  );
}
