import React, { useMemo, useState } from 'react';
import { MingCuteIcon } from './MingCuteIcon';
import type { UrlEncodedParam } from '../../types';

type TreeNode =
  | { kind: 'group'; path: string; segment: string; depth: number; children: TreeNode[]; leafIndexes: number[] }
  | { kind: 'leaf'; index: number; segment: string; depth: number };

type GroupNode = Extract<TreeNode, { kind: 'group' }>;

const BRACKET_KEY = /^([^[\]]+)((?:\[[^[\]]*\])+)$/;

export const hasNestedKeys = (params: UrlEncodedParam[]) => params.some((p) => BRACKET_KEY.test(p.key));

function splitKey(key: string): string[] {
  const match = BRACKET_KEY.exec(key);
  if (!match) return [key];
  return [match[1], ...match[2].slice(1, -1).split('][')];
}

function joinKey(segments: string[]): string {
  return segments[0] + segments.slice(1).map((s) => `[${s}]`).join('');
}

function buildTree(params: UrlEncodedParam[], skipIds: Set<string>): GroupNode {
  const root: GroupNode = { kind: 'group', path: '', segment: '', depth: -1, children: [], leafIndexes: [] };
  params.forEach((param, index) => {
    if (skipIds.has(param.id)) return;
    const segments = splitKey(param.key);
    let group = root;
    group.leafIndexes.push(index);
    segments.slice(0, -1).forEach((segment, depth) => {
      const path = joinKey(segments.slice(0, depth + 1));
      let child = group.children.find((c): c is GroupNode => c.kind === 'group' && c.path === path);
      if (!child) {
        child = { kind: 'group', path, segment, depth, children: [], leafIndexes: [] };
        group.children.push(child);
      }
      child.leafIndexes.push(index);
      group = child;
    });
    group.children.push({ kind: 'leaf', index, segment: segments[segments.length - 1], depth: segments.length - 1 });
  });
  return root;
}

const segmentLabel = (segment: string, depth: number) =>
  depth === 0 ? segment : segment === '' ? '[]' : /^\d+$/.test(segment) ? `[${segment}]` : segment;

const INDENT_PX = 14;

interface UrlEncodedTreeProps {
  params: UrlEncodedParam[];
  onChange: (params: UrlEncodedParam[]) => void;
  readOnly: boolean;
  /** Rows still being typed; kept flat so they don't jump into the tree mid-edit. */
  draftIds: Set<string>;
}

export const UrlEncodedTree: React.FC<UrlEncodedTreeProps> = ({ params, onChange, readOnly, draftIds }) => {
  const tree = useMemo(() => buildTree(params, draftIds), [params, draftIds]);
  const [toggled, setToggled] = useState<Set<string>>(new Set());

  const isOpen = (group: GroupNode) => (group.depth === 0) !== toggled.has(group.path);

  const toggle = (path: string) =>
    setToggled((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const patch = (indexes: number[], change: Partial<UrlEncodedParam>) => {
    const set = new Set(indexes);
    onChange(params.map((p, i) => (set.has(i) ? { ...p, ...change } : p)));
  };

  const remove = (indexes: number[]) => {
    const set = new Set(indexes);
    onChange(params.filter((_, i) => !set.has(i)));
  };

  const renameLeaf = (index: number, segment: string) => {
    const segments = splitKey(params[index].key);
    segments[segments.length - 1] = segment;
    patch([index], { key: joinKey(segments) });
  };

  const inputClass =
    'w-full bg-background border border-border rounded px-2 py-0.5 text-xs text-foreground focus:outline-none focus:border-primary font-mono';

  const renderRow = (
    key: string,
    depth: number,
    checkbox: { checked: boolean; onChange: (checked: boolean) => void },
    label: React.ReactNode,
    value: React.ReactNode,
    onDelete: () => void
  ) => (
    <div key={key} className={`flex gap-2 px-2 py-1 hover:bg-neutral-subtle/50 ${readOnly ? 'items-start' : 'items-center'}`}>
      {!readOnly && (
        <input
          type="checkbox"
          checked={checkbox.checked}
          onChange={(e) => checkbox.onChange(e.target.checked)}
          className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer accent-primary shrink-0"
        />
      )}
      <div className="w-1/3 shrink-0 flex items-center min-w-0" style={{ paddingLeft: depth * INDENT_PX }}>
        {label}
      </div>
      <div className="flex-1 min-w-0">{value}</div>
      {!readOnly && (
        <button
          type="button"
          onClick={onDelete}
          className="text-muted-foreground hover:text-rose-500 p-1 rounded transition-colors cursor-pointer shrink-0"
          title="Remove"
        >
          <MingCuteIcon name="close_line" size={13} />
        </button>
      )}
    </div>
  );

  const renderNode = (node: TreeNode): React.ReactNode => {
    if (node.kind === 'leaf') {
      const param = params[node.index];
      return renderRow(
        param.id,
        node.depth,
        { checked: param.enabled, onChange: (enabled) => patch([node.index], { enabled }) },
        readOnly ? (
          <span className="px-2 py-0.5 text-muted-foreground break-all">{segmentLabel(node.segment, node.depth)}</span>
        ) : (
          <input
            type="text"
            value={node.segment}
            placeholder={node.depth > 0 ? '[]' : 'key_name'}
            onChange={(e) => renameLeaf(node.index, e.target.value)}
            className={inputClass}
          />
        ),
        readOnly ? (
          <span className="block px-2 py-0.5 text-foreground break-all whitespace-pre-wrap">{param.value}</span>
        ) : (
          <input
            type="text"
            value={param.value}
            onChange={(e) => patch([node.index], { value: e.target.value })}
            className={inputClass}
          />
        ),
        () => remove([node.index])
      );
    }

    const open = isOpen(node);
    const leaves = node.leafIndexes.map((i) => params[i]);
    const summary = leaves
      .map((p) => p.value)
      .filter(Boolean)
      .join(' · ');
    return (
      <React.Fragment key={'g:' + node.path}>
        {renderRow(
          'g:' + node.path,
          node.depth,
          { checked: leaves.every((p) => p.enabled), onChange: (enabled) => patch(node.leafIndexes, { enabled }) },
          <button
            type="button"
            onClick={() => toggle(node.path)}
            className={`flex items-center gap-1 min-w-0 text-foreground hover:text-primary cursor-pointer ${readOnly ? 'py-0.5' : ''}`}
          >
            <MingCuteIcon name={open ? 'down_line' : 'right_line'} size={12} className="shrink-0" />
            <span className="truncate">{segmentLabel(node.segment, node.depth)}</span>
            <span className="text-3xs text-muted-foreground tabular-nums">({node.children.length})</span>
          </button>,
          !open && <div className="truncate px-2 py-0.5 text-muted-foreground">{summary}</div>,
          () => remove(node.leafIndexes)
        )}
        {open && node.children.map(renderNode)}
      </React.Fragment>
    );
  };

  const drafts = params.flatMap((param, index) => (draftIds.has(param.id) ? [{ param, index }] : []));

  return (
    <div className="border border-border rounded-lg overflow-hidden bg-surface divide-y divide-border">
      {tree.children.map(renderNode)}
      {drafts.map(({ param, index }) =>
        renderRow(
          param.id,
          0,
          { checked: param.enabled, onChange: (enabled) => patch([index], { enabled }) },
          <input
            type="text"
            placeholder="key_name"
            value={param.key}
            disabled={readOnly}
            onChange={(e) => patch([index], { key: e.target.value })}
            className={inputClass}
          />,
          <input
            type="text"
            placeholder="param_value"
            value={param.value}
            disabled={readOnly}
            onChange={(e) => patch([index], { value: e.target.value })}
            className={inputClass}
          />,
          () => remove([index])
        )
      )}
    </div>
  );
};
