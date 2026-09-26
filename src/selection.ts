// Which rows the highlight may land on. Hints explain; they don't act, so the
// highlight skips them: a highlighted row that does nothing on Enter looks
// like a control and behaves like a dead end.

export interface Selectable {
  type: string;
}

export function isSelectable(row: Selectable | undefined): boolean {
  return row !== undefined && row.type !== "hint";
}

/** First row the highlight can sit on, or -1 when nothing is actionable. */
export function firstSelectable(rows: readonly Selectable[]): number {
  return rows.findIndex(isSelectable);
}

/** Next actionable row in `dir` from `from`; stays put at either end. */
export function step(rows: readonly Selectable[], from: number, dir: 1 | -1): number {
  if (from < 0) return firstSelectable(rows);
  for (let i = from + dir; i >= 0 && i < rows.length; i += dir) {
    if (isSelectable(rows[i])) return i;
  }
  return from;
}

/**
 * The actionable row closest to `at`, preferring the one below: used when the
 * row that was highlighted just left the list (an unpinned favorite).
 */
export function nearestSelectable(rows: readonly Selectable[], at: number): number {
  const start = Math.min(Math.max(at, 0), rows.length - 1);
  for (let i = start; i < rows.length; i++) if (isSelectable(rows[i])) return i;
  for (let i = start - 1; i >= 0; i--) if (isSelectable(rows[i])) return i;
  return -1;
}
