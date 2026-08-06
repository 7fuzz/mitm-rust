import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { clsx } from 'clsx';

interface MarkdownViewerProps {
  content: string;
  className?: string;
  collapsible?: boolean;
  maxCollapsedHeight?: string;
  defaultExpanded?: boolean;
}

export function MarkdownViewer({
  content,
  className,
  collapsible = true,
  maxCollapsedHeight = 'max-h-24',
  defaultExpanded = false,
}: MarkdownViewerProps) {
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  const isLong = Boolean(content && (content.length > 140 || content.split('\n').length > 3));
  const shouldTruncate = collapsible && isLong && !isExpanded;

  const handleCopyCode = (codeText: string) => {
    navigator.clipboard.writeText(codeText);
    setCopiedCode(codeText);
    setTimeout(() => setCopiedCode(null), 2000);
  };

  if (!content || !content.trim()) {
    return (
      <div className="p-4 text-center text-zinc-600 text-xs font-mono border border-dashed border-zinc-900 rounded">
        No documentation written yet. Click &quot;Edit Docs&quot; to add Markdown notes, usage examples, and testing instructions.
      </div>
    );
  }

  return (
    <div className="relative group/markdown">
      <div
        className={clsx(
          "markdown-viewer text-zinc-300 font-mono text-[11px] leading-relaxed space-y-2 transition-all duration-300",
          shouldTruncate && `${maxCollapsedHeight} overflow-hidden relative`,
          className
        )}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            h1: ({ children }) => (
              <h1 className="text-base font-bold text-zinc-100 my-2 pb-1 border-b border-zinc-800">{children}</h1>
            ),
            h2: ({ children }) => (
              <h2 className="text-sm font-bold text-purple-400 my-2">{children}</h2>
            ),
            h3: ({ children }) => (
              <h3 className="text-xs font-bold text-amber-400 my-1.5">{children}</h3>
            ),
            p: ({ children }) => (
              <p className="my-1 text-zinc-300 leading-relaxed">{children}</p>
            ),
            ul: ({ children }) => (
              <ul className="list-disc list-inside my-2 space-y-1 text-zinc-300">{children}</ul>
            ),
            ol: ({ children }) => (
              <ol className="list-decimal list-inside my-2 space-y-1 text-zinc-300">{children}</ol>
            ),
            li: ({ children }) => (
              <li className="leading-relaxed">{children}</li>
            ),
            blockquote: ({ children }) => (
              <blockquote className="my-2 p-2.5 rounded border border-purple-500/30 bg-purple-500/10 text-purple-300 text-[11px] font-mono leading-relaxed">
                {children}
              </blockquote>
            ),
            table: ({ children }) => (
              <div className="my-2 overflow-x-auto rounded border border-zinc-800 bg-zinc-950">
                <table className="w-full text-left text-[11px] font-mono border-collapse">{children}</table>
              </div>
            ),
            thead: ({ children }) => (
              <thead className="bg-zinc-900 border-b border-zinc-800">{children}</thead>
            ),
            th: ({ children }) => (
              <th className="p-2 font-bold text-purple-400 uppercase text-[9px] tracking-wider">{children}</th>
            ),
            td: ({ children }) => (
              <td className="p-2 text-zinc-300 border-b border-zinc-800/50">{children}</td>
            ),
            a: ({ href, children }) => (
              <a href={href} target="_blank" rel="noopener noreferrer" className="text-sky-400 underline hover:text-sky-300 transition-colors">
                {children}
              </a>
            ),
            code({ className, children, ...props }) {
              const match = /language-(\w+)/.exec(className || '');
              const codeString = String(children).replace(/\n$/, '');
              const isInline = !match && !String(children).includes('\n');

              if (isInline) {
                return (
                  <code className="px-1.5 py-0.5 mx-0.5 rounded bg-zinc-900 text-amber-400 border border-zinc-800 text-[10px] font-mono" {...props}>
                    {children}
                  </code>
                );
              }

              const lang = match ? match[1] : 'text';
              const isCopied = copiedCode === codeString;

              return (
                <div className="my-2 rounded border border-zinc-800 bg-zinc-950 overflow-hidden">
                  <div className="flex justify-between items-center px-3 py-1.5 bg-zinc-900/80 border-b border-zinc-800/80 text-[10px] font-mono text-zinc-500">
                    <span className="uppercase font-bold tracking-widest text-purple-400">{lang}</span>
                    <button
                      onClick={() => handleCopyCode(codeString)}
                      className="hover:text-zinc-200 text-zinc-400 px-2 py-0.5 rounded bg-zinc-800/50 hover:bg-zinc-800 transition-all font-sans text-[9px] font-bold uppercase tracking-wider"
                    >
                      {isCopied ? '✓ Copied' : 'Copy'}
                    </button>
                  </div>
                  <pre className="p-3 text-[11px] font-mono text-emerald-text overflow-x-auto whitespace-pre leading-relaxed">
                    <code>{children}</code>
                  </pre>
                </div>
              );
            }
          }}
        >
          {content}
        </ReactMarkdown>

        {shouldTruncate && (
          <div className="absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-zinc-950 via-zinc-950/90 to-transparent pointer-events-none" />
        )}
      </div>

      {collapsible && isLong && (
        <div className="mt-1.5 flex justify-end">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="text-[9px] font-bold uppercase tracking-wider text-purple-400 hover:text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 px-2.5 py-0.5 rounded border border-purple-500/30 transition-all flex items-center gap-1 shadow-sm"
          >
            <span>{isExpanded ? 'Show Less ▴' : 'Show More ▾'}</span>
          </button>
        </div>
      )}
    </div>
  );
}
