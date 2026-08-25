import React from 'react';

interface HexViewerProps {
  content: string;
  className?: string;
}

export const HexViewer: React.FC<HexViewerProps> = ({ content, className = '' }) => {
  // Convert string to bytes
  const bytes = React.useMemo(() => {
    const encoder = new TextEncoder();
    return encoder.encode(content);
  }, [content]);

  // Group into 16-byte rows
  const rows = React.useMemo(() => {
    const r = [];
    for (let i = 0; i < bytes.length; i += 16) {
      r.push(bytes.slice(i, i + 16));
    }
    return r;
  }, [bytes]);

  return (
    <div className={`font-mono text-xs overflow-auto max-h-full p-3 bg-surface text-foreground ${className}`}>
      {rows.length === 0 ? (
        <div className="text-muted-foreground italic">No binary content to render</div>
      ) : (
        <div className="space-y-1">
          {rows.map((row, rowIndex) => {
            const offset = (rowIndex * 16).toString(16).padStart(8, '0');
            const hexParts = [];
            const asciiParts = [];

            for (let i = 0; i < 16; i++) {
              if (i < row.length) {
                const b = row[i];
                hexParts.push(b.toString(16).padStart(2, '0'));
                asciiParts.push(b >= 32 && b <= 126 ? String.fromCharCode(b) : '.');
              } else {
                hexParts.push('  ');
                asciiParts.push(' ');
              }
            }

            return (
              <div key={rowIndex} className="flex gap-4 hover:bg-neutral-subtle px-1 rounded">
                <span className="text-muted-foreground select-none">{offset}</span>
                <span className="text-primary font-medium tracking-wide">
                  {hexParts.slice(0, 8).join(' ')} &nbsp; {hexParts.slice(8, 16).join(' ')}
                </span>
                <span className="text-muted-foreground select-none">|</span>
                <span className="text-foreground tracking-wider">{asciiParts.join('')}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
