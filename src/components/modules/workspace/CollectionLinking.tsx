import React from 'react';
import { useWorkspaceStore } from '../../../stores/useWorkspaceStore';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const CollectionLinking: React.FC = () => {
  const { environments, collectionLinks, toggleCollectionLink } = useWorkspaceStore();
  const { groups } = useRepeaterStore();

  return (
    <div className="bg-surface border border-border rounded-lg p-3 space-y-3 text-xs">
      <div className="flex items-center gap-2">
        <MingCuteIcon name="link_line" size={16} className="text-primary" />
        <span className="font-semibold text-foreground text-sm">Collection & Environment Linking</span>
      </div>
      <p className="text-muted-foreground text-xs">
        Map collection folders to their target execution environments for automated contextual request routing.
      </p>

      <div className="border border-border rounded overflow-hidden">
        <table className="w-full text-left font-mono text-xs">
          <thead>
            <tr className="bg-header border-b border-border text-muted-foreground text-[11px]">
              <th className="px-3 py-1.5 font-medium font-sans">Collection Folder</th>
              {environments.map((env) => (
                <th key={env.id} className="px-3 py-1.5 font-medium text-center font-sans">
                  <span style={{ color: env.color || '#38bdf8' }}>{env.name}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-border bg-surface">
            {groups.length === 0 ? (
              <tr>
                <td colSpan={environments.length + 1} className="px-3 py-4 text-center text-muted-foreground italic font-sans">
                  No collection folders created yet in Repeater
                </td>
              </tr>
            ) : (
              groups.map((grp) => (
                <tr key={grp.id} className="hover:bg-neutral-subtle/50">
                  <td className="px-3 py-2 font-bold text-foreground font-sans flex items-center gap-2">
                    <MingCuteIcon name="folder_line" size={14} className="text-amber-500" />
                    <span>{grp.name}</span>
                  </td>
                  {environments.map((env) => {
                    const isLinked = collectionLinks.some(
                      (l) => l.groupId === grp.id && l.environmentId === env.id
                    );
                    return (
                      <td key={env.id} className="px-3 py-2 text-center">
                        <input
                          type="checkbox"
                          checked={isLinked}
                          onChange={() => toggleCollectionLink(grp.id, env.id)}
                          className="rounded border-border text-primary focus:ring-primary h-4 w-4 cursor-pointer"
                        />
                      </td>
                    );
                  })}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};
