import React from 'react';
import type { DbCell } from '../../../../services/tauri/bridge';
import { MingCuteIcon } from '../../../common/MingCuteIcon';

export interface DbSort {
  column: string;
  desc: boolean;
}

interface DbGridProps {
  columns: string[];
  rows: DbCell[][];
  /** Row number of the first row, for the # gutter when paging */
  firstRowNumber?: number;
  sort?: DbSort | null;
  /** Makes headers clickable */
  onSort?: (column: string) => void;
  selectedIndex?: number | null;
  onRowClick?: (index: number) => void;
  emptyMessage: string;
}

export const DbCellValue: React.FC<{ value: DbCell }> = ({ value }) => {
  if (value === null) return <span className="italic text-muted-foreground/60">NULL</span>;
  if (typeof value === 'number') return <span className="text-sky-500">{value}</span>;
  return <>{value}</>;
};

/** Read-only result grid with a sticky header, row numbers, and optional sorting and row selection. */
export const DbGrid: React.FC<DbGridProps> = ({
  columns,
  rows,
  firstRowNumber = 1,
  sort,
  onSort,
  selectedIndex,
  onRowClick,
  emptyMessage,
}) => (
  <div className="h-full overflow-auto">
    <table className="w-max min-w-full text-left font-mono text-2xs border-collapse">
      <thead className="bg-header text-muted-foreground sticky top-0 z-10">
        <tr>
          <th className="px-2 py-1.5 font-semibold font-sans text-right border-b border-r border-border w-10">#</th>
          {columns.map((col) => {
            const isSorted = sort?.column === col;
            return (
              <th
                key={col}
                onClick={onSort ? () => onSort(col) : undefined}
                className={`px-3 py-1.5 font-semibold font-sans border-b border-border whitespace-nowrap select-none ${
                  onSort ? 'cursor-pointer hover:text-foreground' : ''
                } ${isSorted ? 'text-foreground' : ''}`}
                title={onSort ? 'Click to sort' : undefined}
              >
                <span className="inline-flex items-center gap-1">
                  {col}
                  {isSorted && <MingCuteIcon name={sort.desc ? 'arrow_down_line' : 'arrow_up_line'} size={11} className="text-primary" />}
                </span>
              </th>
            );
          })}
        </tr>
      </thead>
      <tbody>
        {rows.length === 0 ? (
          <tr>
            <td colSpan={columns.length + 1} className="px-3 py-8 text-center text-muted-foreground italic font-sans">
              {emptyMessage}
            </td>
          </tr>
        ) : (
          rows.map((row, rIdx) => (
            <tr
              key={rIdx}
              onClick={onRowClick ? () => onRowClick(rIdx) : undefined}
              className={`border-b border-border/60 ${onRowClick ? 'cursor-pointer' : ''} ${
                selectedIndex === rIdx ? 'bg-primary/10' : 'hover:bg-neutral-subtle/60'
              }`}
            >
              <td className="px-2 py-1 text-right text-muted-foreground/70 border-r border-border tabular-nums">
                {firstRowNumber + rIdx}
              </td>
              {row.map((cell, cIdx) => (
                <td key={cIdx} className="px-3 py-1 max-w-[320px] truncate text-foreground" title={cell === null ? 'NULL' : String(cell)}>
                  <DbCellValue value={cell} />
                </td>
              ))}
            </tr>
          ))
        )}
      </tbody>
    </table>
  </div>
);
