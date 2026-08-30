import React from 'react';
import { MingCuteIcon } from './MingCuteIcon';

interface MarkdownViewerProps {
  content: string;
  onEdit?: () => void;
  className?: string;
}

export const MarkdownViewer: React.FC<MarkdownViewerProps> = ({
  content,
  onEdit,
  className = '',
}) => {
  if (!content || !content.trim()) {
    return (
      <div className={`h-full flex flex-col items-center justify-center text-muted-foreground p-6 text-xs text-center ${className}`}>
        <MingCuteIcon name="file_text_line" size={36} className="opacity-30 mb-2" />
        <p className="font-medium text-foreground mb-1">No documentation available</p>
        <p className="text-muted-foreground mb-3 max-w-sm">
          Add Markdown documentation for this request to describe its purpose, parameters, authentication, or example responses.
        </p>
        {onEdit && (
          <button
            onClick={onEdit}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-primary text-primary-foreground font-semibold hover:bg-primary/90 transition-colors cursor-pointer text-xs shadow-xs"
          >
            <MingCuteIcon name="edit_line" size={13} />
            Write Documentation
          </button>
        )}
      </div>
    );
  }

  // Parse markdown into rendered React elements
  const renderMarkdown = (text: string) => {
    const lines = text.split('\n');
    const elements: React.ReactNode[] = [];
    let inCodeBlock = false;
    let codeBlockLang = '';
    let codeBlockLines: string[] = [];
    let inTable = false;
    let tableRows: string[][] = [];
    let inList = false;
    let listType: 'ul' | 'ol' = 'ul';
    let listItems: string[] = [];

    const flushList = () => {
      if (inList && listItems.length > 0) {
        if (listType === 'ul') {
          elements.push(
            <ul key={`ul-${elements.length}`} className="list-disc list-inside my-2 space-y-1 text-foreground/90 ml-2">
              {listItems.map((item, idx) => (
                <li key={idx} className="leading-relaxed">
                  {renderInlineMarkdown(item)}
                </li>
              ))}
            </ul>
          );
        } else {
          elements.push(
            <ol key={`ol-${elements.length}`} className="list-decimal list-inside my-2 space-y-1 text-foreground/90 ml-2">
              {listItems.map((item, idx) => (
                <li key={idx} className="leading-relaxed">
                  {renderInlineMarkdown(item)}
                </li>
              ))}
            </ol>
          );
        }
        listItems = [];
        inList = false;
      }
    };

    const flushTable = () => {
      if (inTable && tableRows.length > 0) {
        const headerRow = tableRows[0];
        const bodyRows = tableRows.slice(1);
        elements.push(
          <div key={`table-wrapper-${elements.length}`} className="my-3 overflow-x-auto rounded border border-border">
            <table className="min-w-full divide-y divide-border text-xs">
              <thead className="bg-header">
                <tr>
                  {headerRow.map((cell, cIdx) => (
                    <th key={cIdx} className="px-3 py-2 text-left font-semibold text-foreground">
                      {renderInlineMarkdown(cell.trim())}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-border bg-surface">
                {bodyRows.map((row, rIdx) => (
                  <tr key={rIdx} className="hover:bg-neutral-subtle/50 transition-colors">
                    {row.map((cell, cIdx) => (
                      <td key={cIdx} className="px-3 py-1.5 text-foreground/90">
                        {renderInlineMarkdown(cell.trim())}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
        tableRows = [];
        inTable = false;
      }
    };

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmed = line.trim();

      // Fenced Code Block
      if (trimmed.startsWith('```')) {
        flushList();
        flushTable();
        if (inCodeBlock) {
          const codeContent = codeBlockLines.join('\n');
          const lang = codeBlockLang;
          elements.push(
            <div key={`codeblock-${elements.length}`} className="my-3 rounded-lg border border-border overflow-hidden bg-background">
              <div className="flex items-center justify-between px-3 py-1 bg-header border-b border-border text-[11px] font-mono text-muted-foreground">
                <span>{lang || 'text'}</span>
                <button
                  onClick={() => navigator.clipboard.writeText(codeContent)}
                  className="flex items-center gap-1 hover:text-foreground transition-colors cursor-pointer"
                  title="Copy code"
                >
                  <MingCuteIcon name="copy_line" size={12} />
                  <span>Copy</span>
                </button>
              </div>
              <pre className="p-3 text-xs font-mono overflow-x-auto leading-relaxed text-foreground whitespace-pre">
                <code>{codeContent}</code>
              </pre>
            </div>
          );
          codeBlockLines = [];
          inCodeBlock = false;
          codeBlockLang = '';
        } else {
          inCodeBlock = true;
          codeBlockLang = trimmed.slice(3).trim();
        }
        continue;
      }

      if (inCodeBlock) {
        codeBlockLines.push(line);
        continue;
      }

      // Markdown Tables
      if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
        flushList();
        const cells = trimmed
          .slice(1, -1)
          .split('|')
          .map((c) => c.trim());

        // Skip separator line (e.g. |---|---|)
        if (cells.every((c) => /^:?-+:?$/.test(c))) {
          continue;
        }

        inTable = true;
        tableRows.push(cells);
        continue;
      } else {
        flushTable();
      }

      // Unordered List
      if (/^[-*+]\s+/.test(trimmed)) {
        if (inList && listType !== 'ul') flushList();
        inList = true;
        listType = 'ul';
        const itemText = trimmed.replace(/^[-*+]\s+/, '');
        listItems.push(itemText);
        continue;
      }

      // Ordered List
      if (/^\d+\.\s+/.test(trimmed)) {
        if (inList && listType !== 'ol') flushList();
        inList = true;
        listType = 'ol';
        const itemText = trimmed.replace(/^\d+\.\s+/, '');
        listItems.push(itemText);
        continue;
      }

      flushList();

      // Empty Line
      if (!trimmed) {
        elements.push(<div key={`spacer-${elements.length}`} className="h-2" />);
        continue;
      }

      // Headings
      if (trimmed.startsWith('# ')) {
        elements.push(
          <h1 key={`h1-${elements.length}`} className="text-lg font-bold text-foreground mt-4 mb-2 pb-1 border-b border-border">
            {renderInlineMarkdown(trimmed.slice(2))}
          </h1>
        );
        continue;
      }
      if (trimmed.startsWith('## ')) {
        elements.push(
          <h2 key={`h2-${elements.length}`} className="text-base font-bold text-foreground mt-3 mb-1.5 pb-1 border-b border-border/60">
            {renderInlineMarkdown(trimmed.slice(3))}
          </h2>
        );
        continue;
      }
      if (trimmed.startsWith('### ')) {
        elements.push(
          <h3 key={`h3-${elements.length}`} className="text-sm font-semibold text-foreground mt-2.5 mb-1 text-primary">
            {renderInlineMarkdown(trimmed.slice(4))}
          </h3>
        );
        continue;
      }
      if (trimmed.startsWith('#### ')) {
        elements.push(
          <h4 key={`h4-${elements.length}`} className="text-xs font-semibold text-foreground mt-2 mb-1">
            {renderInlineMarkdown(trimmed.slice(5))}
          </h4>
        );
        continue;
      }

      // Blockquotes
      if (trimmed.startsWith('> ')) {
        elements.push(
          <blockquote
            key={`bq-${elements.length}`}
            className="border-l-2 border-primary/70 bg-neutral-subtle/30 pl-3 py-1.5 my-2 italic text-muted-foreground rounded-r text-xs"
          >
            {renderInlineMarkdown(trimmed.slice(2))}
          </blockquote>
        );
        continue;
      }

      // Horizontal Rule
      if (/^(\*\*\*|---|___)$/.test(trimmed)) {
        elements.push(<hr key={`hr-${elements.length}`} className="my-3 border-border" />);
        continue;
      }

      // Standard Paragraph
      elements.push(
        <p key={`p-${elements.length}`} className="my-1.5 leading-relaxed text-foreground/90 text-xs">
          {renderInlineMarkdown(line)}
        </p>
      );
    }

    flushList();
    flushTable();

    return elements;
  };

  // Inline markdown parser (bold, italic, inline code, link, strike, tasks)
  const renderInlineMarkdown = (text: string): React.ReactNode => {
    let processed = text;
    let isTaskChecked: boolean | null = null;

    if (/^\[ \]\s+/.test(processed)) {
      isTaskChecked = false;
      processed = processed.replace(/^\[ \]\s+/, '');
    } else if (/^\[x\]\s+/i.test(processed)) {
      isTaskChecked = true;
      processed = processed.replace(/^\[x\]\s+/i, '');
    }

    const regex = /(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|~~[^~]+~~|\[[^\]]+\]\([^)]+\))/g;
    const parts = processed.split(regex);

    const inlineNodes = parts.map((part, idx) => {
      if (!part) return null;

      // Inline code
      if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
        return (
          <code
            key={idx}
            className="px-1.5 py-0.5 rounded bg-background border border-border font-mono text-[11px] text-primary"
          >
            {part.slice(1, -1)}
          </code>
        );
      }

      // Bold
      if (part.startsWith('**') && part.endsWith('**') && part.length >= 4) {
        return (
          <strong key={idx} className="font-bold text-foreground">
            {part.slice(2, -2)}
          </strong>
        );
      }

      // Italic
      if (part.startsWith('*') && part.endsWith('*') && part.length >= 2) {
        return (
          <em key={idx} className="italic text-foreground/90">
            {part.slice(1, -1)}
          </em>
        );
      }

      // Strikethrough
      if (part.startsWith('~~') && part.endsWith('~~') && part.length >= 4) {
        return (
          <span key={idx} className="line-through opacity-70">
            {part.slice(2, -2)}
          </span>
        );
      }

      // Links: [text](url)
      const linkMatch = part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      if (linkMatch) {
        return (
          <a
            key={idx}
            href={linkMatch[2]}
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary underline hover:opacity-80 transition-opacity"
          >
            {linkMatch[1]}
          </a>
        );
      }

      return part;
    });

    if (isTaskChecked !== null) {
      return (
        <span className="flex items-center gap-1.5">
          <input
            type="checkbox"
            checked={isTaskChecked}
            readOnly
            className="accent-primary rounded cursor-default"
          />
          <span>{inlineNodes}</span>
        </span>
      );
    }

    return inlineNodes;
  };

  return (
    <div className={`h-full overflow-y-auto px-4 py-3 text-xs leading-relaxed text-foreground select-text font-sans ${className}`}>
      {renderMarkdown(content)}
    </div>
  );
};
