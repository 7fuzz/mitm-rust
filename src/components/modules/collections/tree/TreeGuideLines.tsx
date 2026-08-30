import React from 'react';

interface TreeGuideLinesProps {
  depth: number;
  isLast: boolean;
  ancestorIsLast?: boolean[];
}

export const TreeGuideLines: React.FC<TreeGuideLinesProps> = ({
  depth,
  isLast,
  ancestorIsLast = [],
}) => {
  if (depth <= 0) return null;

  return (
    <div className="flex self-stretch items-stretch shrink-0 select-none pointer-events-none h-full">
      {/* Ancestor Vertical Continuation Lines (│   ) */}
      {Array.from({ length: depth - 1 }).map((_, i) => {
        const isAncestorLast = ancestorIsLast[i] ?? false;
        return (
          <div key={i} className="w-4 h-full relative shrink-0">
            {!isAncestorLast && (
              <div className="absolute top-0 bottom-0 left-[7px] w-[1.5px] bg-zinc-400 dark:bg-zinc-500" />
            )}
          </div>
        );
      })}

      {/* Current Level Branch Line (├── or └──) */}
      <div className="w-4 h-full relative shrink-0">
        {/* Top half vertical line */}
        <div className="absolute top-0 bottom-1/2 left-[7px] w-[1.5px] bg-zinc-400 dark:bg-zinc-500" />

        {/* Bottom half vertical line (only if not last sibling) */}
        {!isLast && (
          <div className="absolute top-1/2 bottom-0 left-[7px] w-[1.5px] bg-zinc-400 dark:bg-zinc-500" />
        )}

        {/* Horizontal arm connecting to the node icon */}
        <div className="absolute top-1/2 left-[7px] right-0 h-[1.5px] -translate-y-1/2 bg-zinc-400 dark:bg-zinc-500" />
      </div>
    </div>
  );
};
