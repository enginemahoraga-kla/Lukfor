export type EntryKind = "app" | "file" | "folder";

export interface SearchResult {
  name: string;
  path: string;
  kind: EntryKind;
  score: number;
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
