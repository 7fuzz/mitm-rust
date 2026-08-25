import React, { useRef } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import type { MultipartField } from '../../types';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';

interface MultipartEditorProps {
  fields: MultipartField[];
  onChange: (fields: MultipartField[]) => void;
  readOnly?: boolean;
}

export const MultipartEditor: React.FC<MultipartEditorProps> = ({
  fields,
  onChange,
  readOnly = false,
}) => {
  const hiddenFileInputRef = useRef<HTMLInputElement>(null);
  const targetIndexRef = useRef<number | null>(null);

  const handleAddField = () => {
    const newField: MultipartField = {
      id: 'field_' + Date.now() + '_' + Math.random().toString(36).substring(2, 6),
      enabled: true,
      key: '',
      value: '',
      type: 'text',
    };
    onChange([...fields, newField]);
  };

  const handleFieldChange = (index: number, patch: Partial<MultipartField>) => {
    const updated = [...fields];
    updated[index] = { ...updated[index], ...patch };
    onChange(updated);
  };

  const handleDeleteField = (index: number) => {
    const updated = fields.filter((_, i) => i !== index);
    onChange(updated);
  };

  const handlePickFileNative = async (index: number) => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        title: 'Select Attachment File',
      });

      if (selected && typeof selected === 'string') {
        const filePath = selected;
        const fileName = filePath.split(/[/\\]/).pop() || 'file';
        const ext = fileName.split('.').pop()?.toLowerCase() || '';

        let contentType = 'application/octet-stream';
        if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) contentType = `image/${ext === 'jpg' ? 'jpeg' : ext}`;
        else if (ext === 'pdf') contentType = 'application/pdf';
        else if (ext === 'json') contentType = 'application/json';
        else if (ext === 'txt') contentType = 'text/plain';

        handleFieldChange(index, {
          type: 'file_path',
          value: filePath,
          file_name: fileName,
          content_type: contentType,
        });
      }
    } catch {
      // Fallback to HTML5 file input if Tauri dialog is not available
      targetIndexRef.current = index;
      hiddenFileInputRef.current?.click();
    }
  };

  const handleHTML5FileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    const index = targetIndexRef.current;
    if (!file || index === null) return;

    const reader = new FileReader();
    reader.onload = (evt) => {
      const dataUrl = evt.target?.result as string;
      handleFieldChange(index, {
        type: 'base64',
        value: dataUrl,
        file_name: file.name,
        content_type: file.type || 'application/octet-stream',
      });
    };
    reader.readAsDataURL(file);

    // Reset input
    e.target.value = '';
    targetIndexRef.current = null;
  };

  return (
    <div className="w-full flex flex-col gap-2 font-mono text-xs select-none">
      <input
        ref={hiddenFileInputRef}
        type="file"
        onChange={handleHTML5FileSelect}
        className="hidden"
      />

      <div className="border border-border rounded-lg overflow-hidden">
        <table className="w-full text-left">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
              <th className="w-8 px-2 py-1.5 text-center">En</th>
              <th className="w-24 px-2 py-1.5 font-medium">Type</th>
              <th className="w-1/3 px-3 py-1.5 font-medium">Key Name</th>
              <th className="px-3 py-1.5 font-medium">Value / Attachment</th>
              {!readOnly && <th className="w-8 px-2 py-1.5 text-center"></th>}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {fields.length === 0 ? (
              <tr>
                <td colSpan={readOnly ? 4 : 5} className="px-3 py-6 text-center text-muted-foreground italic font-sans text-xs">
                  No multipart form-data fields added.
                </td>
              </tr>
            ) : (
              fields.map((field, idx) => (
                <tr key={field.id || idx} className="hover:bg-neutral-subtle/50">
                  {/* Enable Checkbox */}
                  <td className="px-2 py-1 text-center">
                    <input
                      type="checkbox"
                      checked={field.enabled}
                      disabled={readOnly}
                      onChange={(e) => handleFieldChange(idx, { enabled: e.target.checked })}
                      className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer accent-primary"
                    />
                  </td>

                  {/* Field Type Selector */}
                  <td className="px-1.5 py-1">
                    <select
                      value={field.type === 'text' ? 'text' : 'file'}
                      disabled={readOnly}
                      onChange={(e) => {
                        const newMode = e.target.value;
                        if (newMode === 'text') {
                          handleFieldChange(idx, { type: 'text', value: '', file_name: undefined, content_type: undefined });
                        } else {
                          handlePickFileNative(idx);
                        }
                      }}
                      className="w-full bg-background border border-border rounded px-1.5 py-0.5 text-[11px] text-foreground focus:outline-none cursor-pointer"
                    >
                      <option value="text">Text</option>
                      <option value="file">File</option>
                    </select>
                  </td>

                  {/* Key Name Input */}
                  <td className="px-2 py-1">
                    <input
                      type="text"
                      placeholder="field_name"
                      value={field.key}
                      disabled={readOnly}
                      onChange={(e) => handleFieldChange(idx, { key: e.target.value })}
                      className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary"
                    />
                  </td>

                  {/* Value / File Picker Cell */}
                  <td className="px-2 py-1">
                    {field.type === 'text' ? (
                      <input
                        type="text"
                        placeholder="Field text value..."
                        value={field.value}
                        disabled={readOnly}
                        onChange={(e) => handleFieldChange(idx, { value: e.target.value })}
                        className="w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary"
                      />
                    ) : (
                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          disabled={readOnly}
                          onClick={() => handlePickFileNative(idx)}
                          className="px-2 py-0.5 bg-header hover:bg-neutral-subtle border border-border rounded text-[11px] font-semibold text-foreground cursor-pointer transition-colors flex items-center gap-1 shrink-0"
                        >
                          <MingCuteIcon name="folder_open_line" size={13} className="text-amber-500" />
                          <span>Select File</span>
                        </button>
                        <span className="truncate text-xs text-foreground/90 font-mono">
                          {field.file_name || field.value || 'No file selected'}
                        </span>
                        {field.content_type && (
                          <span className="text-[10px] text-muted-foreground bg-background px-1.5 py-0.2 rounded border border-border shrink-0">
                            {field.content_type}
                          </span>
                        )}
                      </div>
                    )}
                  </td>

                  {/* Delete Button */}
                  {!readOnly && (
                    <td className="px-2 py-1 text-center">
                      <button
                        type="button"
                        onClick={() => handleDeleteField(idx)}
                        className="text-muted-foreground hover:text-rose-500 p-1 rounded transition-colors cursor-pointer"
                        title="Remove Field"
                      >
                        <MingCuteIcon name="close_line" size={13} />
                      </button>
                    </td>
                  )}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {!readOnly && (
        <button
          type="button"
          onClick={handleAddField}
          className="px-3 py-1 rounded bg-background hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs cursor-pointer transition-colors flex items-center gap-1 self-start font-sans"
        >
          <MingCuteIcon name="plus_line" size={13} />
          <span>Add Form Field</span>
        </button>
      )}
    </div>
  );
};
