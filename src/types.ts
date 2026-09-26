export type EntryKind = "app" | "file" | "folder";

export interface SearchResult {
  name: string;
  path: string;
  kind: EntryKind;
  score: number;
  favorite: boolean;
}

export interface FavoriteView {
  name: string;
  path: string;
  kind: EntryKind;
  /** No longer in the index; listed anyway so it can be unpinned. */
  missing: boolean;
}

export interface ClipEntry {
  text: string;
  at: number; // unix seconds
}

export interface Status {
  hotkey: string;
  indexed: number;
  indexing: boolean;
}
