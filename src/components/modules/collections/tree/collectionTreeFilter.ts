import type { CollectionTreeItem } from '../../../../services/tauri/bridge';

// Recursive tree filter supporting name, URL, method, and description
export const filterTree = (items: CollectionTreeItem[], query: string): CollectionTreeItem[] => {
  const q = query.trim().toLowerCase();
  if (!q) return items;

  const terms = q.split(/\s+/).filter(Boolean);

  const matchesAllTerms = (text: string) => {
    const lower = text.toLowerCase();
    return terms.every((t) => lower.includes(t));
  };

  return items
    .map((item) => {
      const matchingChildren = filterTree(item.children || [], query);
      const matchingRequests = (item.requests || []).filter((r) => {
        const combined = `${r.name} ${r.url || ''} ${r.method || ''} ${r.description || ''}`;
        return matchesAllTerms(combined);
      });

      const folderCombined = `${item.name} ${item.description || ''}`;
      const folderMatches = matchesAllTerms(folderCombined);

      if (folderMatches) {
        return item;
      }

      if (matchingChildren.length > 0 || matchingRequests.length > 0) {
        return {
          ...item,
          children: matchingChildren,
          requests: matchingRequests,
        };
      }

      return null;
    })
    .filter((item): item is CollectionTreeItem => item !== null);
};
