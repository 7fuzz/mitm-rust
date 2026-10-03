import React, { useEffect, useMemo, useRef } from 'react';
import { DiffEditor, DiffOnMount } from '@monaco-editor/react';
import { useSettingsStore } from '../../stores/useSettingsStore';

type DiffEditorInstance = Parameters<DiffOnMount>[0];

interface DiffCodeEditorProps {
  original: string;
  modified: string;
  onOriginalChange: (value: string) => void;
  onModifiedChange: (value: string) => void;
  language?: string;
  inline?: boolean;
}

/**
 * Both sides editable. Values are only pushed into the models when they differ,
 * because @monaco-editor/react resets the original model on every prop change,
 * which would move the cursor while typing.
 */
export const DiffCodeEditor: React.FC<DiffCodeEditorProps> = ({
  original,
  modified,
  onOriginalChange,
  onModifiedChange,
  language = 'json',
  inline = false,
}) => {
  const { theme } = useSettingsStore();
  const editorRef = useRef<DiffEditorInstance | null>(null);
  const initial = useRef({ original, modified });
  const handlers = useRef({ onOriginalChange, onModifiedChange });
  handlers.current = { onOriginalChange, onModifiedChange };

  const handleMount: DiffOnMount = (editor) => {
    editorRef.current = editor;
    const originalEditor = editor.getOriginalEditor();
    const modifiedEditor = editor.getModifiedEditor();
    originalEditor.onDidChangeModelContent(() => handlers.current.onOriginalChange(originalEditor.getValue()));
    modifiedEditor.onDidChangeModelContent(() => handlers.current.onModifiedChange(modifiedEditor.getValue()));
  };

  useEffect(() => {
    const editor = editorRef.current?.getOriginalEditor();
    if (editor && editor.getValue() !== original) editor.setValue(original);
  }, [original]);

  useEffect(() => {
    const editor = editorRef.current?.getModifiedEditor();
    if (editor && editor.getValue() !== modified) editor.setValue(modified);
  }, [modified]);

  const options = useMemo(
    () => ({
      originalEditable: true,
      renderSideBySide: !inline,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      fontSize: 12,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Menlo', 'Monaco', monospace",
      lineNumbersMinChars: 3,
      wordWrap: 'on' as const,
      automaticLayout: true,
      padding: { top: 8, bottom: 8 },
      renderOverviewRuler: false,
    }),
    [inline]
  );

  return (
    <DiffEditor
      height="100%"
      language={language}
      original={initial.current.original}
      modified={initial.current.modified}
      theme={theme === 'dark' ? 'vs-dark' : 'light'}
      onMount={handleMount}
      loading={<div className="h-full flex items-center justify-center text-xs font-mono text-muted-foreground">Loading editor...</div>}
      options={options}
    />
  );
};
