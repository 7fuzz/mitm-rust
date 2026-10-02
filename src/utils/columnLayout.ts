/** Persistable layout of a table's columns: display order, pixel widths, and hidden columns. */
export interface ColumnLayout {
  order: string[];
  widths: Record<string, number>;
  hidden: string[];
}

export interface ColumnDefault {
  id: string;
  width: number;
  hidden?: boolean;
}

export const buildDefaultColumnLayout = (columns: ColumnDefault[]): ColumnLayout => ({
  order: columns.map((c) => c.id),
  widths: Object.fromEntries(columns.map((c) => [c.id, c.width])),
  hidden: columns.filter((c) => c.hidden).map((c) => c.id),
});

/**
 * Reconciles a stored layout with the current column set: unknown columns are dropped,
 * columns added since the layout was saved are appended with their defaults.
 */
export const normalizeColumnLayout = (stored: unknown, defaults: ColumnLayout): ColumnLayout => {
  if (!stored || typeof stored !== "object") return defaults;
  const s = stored as Partial<ColumnLayout>;
  const known = new Set(defaults.order);

  const order = (Array.isArray(s.order) ? s.order : []).filter((id) => known.has(id));
  const seen = new Set(order);
  for (const id of defaults.order) {
    if (!seen.has(id)) {
      order.push(id);
    }
  }

  const widths: Record<string, number> = { ...defaults.widths };
  if (s.widths && typeof s.widths === "object") {
    for (const [id, w] of Object.entries(s.widths)) {
      if (known.has(id) && typeof w === "number" && Number.isFinite(w)) widths[id] = w;
    }
  }

  const hidden = Array.isArray(s.hidden) ? s.hidden.filter((id) => known.has(id)) : defaults.hidden;
  return { order, widths, hidden };
};

export const moveColumn = (order: string[], id: string, targetId: string, after: boolean): string[] => {
  if (id === targetId) return order;
  const without = order.filter((c) => c !== id);
  const idx = without.indexOf(targetId);
  if (idx === -1) return order;
  without.splice(after ? idx + 1 : idx, 0, id);
  return without;
};
