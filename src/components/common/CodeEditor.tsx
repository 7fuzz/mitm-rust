import React from 'react';
import Editor, { OnChange, loader } from '@monaco-editor/react';
import { useSettingsStore } from '../../stores/useSettingsStore';

// Configure CDN loader fallback
loader.config({
  paths: {
    vs: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.43.0/min/vs',
  },
});

interface CodeEditorProps {
  value: string;
  onChange?: (value: string) => void;
  language?: string;
  readOnly?: boolean;
  height?: string | number;
  minHeight?: string;
  className?: string;
}

export const CodeEditor: React.FC<CodeEditorProps> = ({
  value,
  onChange,
  language = 'json',
  readOnly = false,
  height = '100%',
  minHeight = '150px',
  className = '',
}) => {
  const { theme } = useSettingsStore();
  const editorTheme = theme === 'dark' ? 'vs-dark' : 'light';

  const handleChange: OnChange = (val) => {
    if (onChange && val !== undefined) {
      onChange(val);
    }
  };

  return (
    <div
      className={`relative w-full h-full min-h-[${minHeight}] border border-border rounded overflow-hidden bg-surface ${className}`}
    >
      <Editor
        height={height}
        language={language}
        value={value}
        theme={editorTheme}
        onChange={handleChange}
        loading={
          <div className="h-full flex items-center justify-center p-4 text-xs font-mono text-muted-foreground bg-surface italic">
            Loading editor...
          </div>
        }
        options={{
          readOnly,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 12,
          fontFamily: "'JetBrains Mono', 'Fira Code', 'Menlo', 'Monaco', monospace",
          lineNumbersMinChars: 3,
          wordWrap: 'on',
          automaticLayout: true,
          padding: { top: 8, bottom: 8 },
          folding: true,
        }}
      />
    </div>
  );
};
