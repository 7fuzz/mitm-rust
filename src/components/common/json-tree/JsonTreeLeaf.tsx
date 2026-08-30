import React, { useState } from 'react';
import { Base64LeafActions } from './Base64LeafActions';
import { selectTextOf } from './jsonTreeSerializer';

export const HighlightText: React.FC<{ text: string; query?: string }> = ({ text, query }) => {
  if (!query) return <>{text}</>;
  const parts = text.toString().split(new RegExp(`(${query})`, 'gi'));
  return (
    <>
      {parts.map((part, i) =>
        part.toLowerCase() === query.toLowerCase() ? (
          <span key={i} className="bg-amber-500/40 text-amber-200 rounded px-0.5 font-bold">
            {part}
          </span>
        ) : (
          part
        )
      )}
    </>
  );
};

interface JsonTreeLeafProps {
  label?: string;
  value: unknown;
  isLast?: boolean;
  searchTerm?: string;
  filterMode?: boolean;
  shouldForceShow?: boolean;
  valueMatches: boolean;
  labelMatches: boolean;
}

export const JsonTreeLeaf: React.FC<JsonTreeLeafProps> = ({
  label,
  value,
  isLast = true,
  searchTerm = '',
  filterMode = false,
  shouldForceShow = false,
  valueMatches,
  labelMatches,
}) => {
  const [isLongTextExpanded, setIsLongTextExpanded] = useState(false);

  if (filterMode && searchTerm && !shouldForceShow && !valueMatches && !labelMatches) {
    return null;
  }

  let valueColor = 'text-foreground';
  let formattedValue = String(value);
  const isString = typeof value === 'string';

  if (isString) valueColor = 'text-emerald-400 font-mono';
  else if (typeof value === 'number') valueColor = 'text-amber-400 font-mono';
  else if (typeof value === 'boolean') valueColor = 'text-purple-400 font-mono';
  else if (value === null) {
    valueColor = 'text-rose-400 font-mono';
    formattedValue = 'null';
  }

  const isLongText = isString && (value as string).length > 200;
  const shouldShowFullText = isLongTextExpanded || (searchTerm && valueMatches);
  const displayedText =
    isLongText && !shouldShowFullText ? (value as string).substring(0, 100) : formattedValue;

  return (
    <div className="font-mono text-xs leading-relaxed flex items-start group py-0.5">
      <div className="flex-1 min-w-0 flex items-center flex-wrap">
        {label && (
          <span
            className="text-primary mr-1 whitespace-nowrap font-semibold cursor-text"
            onDoubleClick={(e) => {
              e.preventDefault();
              selectTextOf(e.currentTarget);
            }}
          >
            &quot;<HighlightText text={label} query={searchTerm} />&quot;:
          </span>
        )}
        <span
          className={`${valueColor} cursor-text`}
          onDoubleClick={(e) => {
            e.preventDefault();
            selectTextOf(e.currentTarget);
          }}
        >
          {isString && '"'}
          <HighlightText text={displayedText} query={searchTerm} />
          {isLongText && !shouldShowFullText && '...'}
          {isString && '"'}
        </span>

        {isLongText && (
          <button
            type="button"
            onClick={() => setIsLongTextExpanded(!shouldShowFullText)}
            className="ml-2 text-[10px] font-sans text-muted-foreground hover:text-foreground bg-background border border-border px-1.5 py-0.5 rounded cursor-pointer shrink-0"
          >
            {shouldShowFullText ? 'Collapse' : `Expand (${(value as string).length} chars)`}
          </button>
        )}

        {isString && <Base64LeafActions value={value as string} label={label} />}

        {!isLast && <span className="text-muted-foreground ml-0.5">,</span>}
      </div>
    </div>
  );
};
