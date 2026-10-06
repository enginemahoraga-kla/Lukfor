// What a result row can be, and how each kind is drawn.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Glyph, StarGlyph } from "./Glyph";
import { POWER_TEXT, type PowerAction } from "./power";
import type { ClipEntry, FavoriteView, SearchResult } from "./types";

export type Row =
  | { type: "calc"; expr: string; result: string }
  | { type: "web"; query: string }
  | { type: "clip"; entry: ClipEntry }
  | { type: "entry"; entry: SearchResult }
  | { type: "favorite"; fav: FavoriteView }
  | { type: "reindex" }
  | { type: "power"; action: PowerAction }
  | { type: "hint"; text: string };

/** The path a row stands for, if it is something that can be pinned. */
export function pinnablePath(row: Row | undefined): string | null {
  if (row?.type === "entry") return row.entry.path;
  if (row?.type === "favorite") return row.fav.path;
  return null;
}

// Icon data URLs per path; null = backend has no icon (keep the glyph).
const iconCache = new Map<string, string | null>();

function AppIcon({ path }: { path: string }) {
  const [src, setSrc] = useState<string | null>(iconCache.get(path) ?? null);
  useEffect(() => {
    if (iconCache.has(path)) {
      setSrc(iconCache.get(path) ?? null);
      return;
    }
    let alive = true;
    invoke<string | null>("get_icon", { path })
      .then((url) => {
        iconCache.set(path, url);
        if (alive) setSrc(url);
      })
      .catch(() => {
        iconCache.set(path, null);
      });
    return () => {
      alive = false;
    };
  }, [path]);
  if (!src) return <Glyph kind="app" />;
  return (
    <span className="icon" aria-hidden="true">
      <img className="appicon" src={src} alt="" draggable={false} />
    </span>
  );
}

// The star is the one mouse path to pinning; Ctrl+D is the keyboard one, so
// the button stays out of the Tab order and focus never leaves the input.
// Screen readers hear "pinned" from the row itself, so the button is hidden
// from them rather than being a control nested inside a list option.
function PinButton({ pinned, onToggle }: { pinned: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      className={`pin ${pinned ? "on" : ""}`}
      aria-hidden="true"
      title={pinned ? "Unpin (Ctrl+D)" : "Pin to favorites (Ctrl+D)"}
      tabIndex={-1}
      onMouseDown={(e) => {
        // Don't let the row underneath treat this as "open".
        e.preventDefault();
        e.stopPropagation();
        onToggle();
      }}
    >
      <StarGlyph filled={pinned} />
    </button>
  );
}

export function rowKey(r: Row, i: number): string {
  return `${r.type}-${i}`;
}

export function RowContent({
  row,
  indexing,
  armed,
  onTogglePin,
}: {
  row: Row;
  indexing: boolean;
  /** The power action waiting for its confirming Enter, if any. */
  armed: PowerAction | null;
  onTogglePin: (path: string) => void;
}) {
  switch (row.type) {
    case "calc":
      return (
        <>
          <Glyph kind="calc" />
          <span className="name result">{row.result}</span>
          <span className="sub">Enter to copy</span>
        </>
      );
    case "web":
      return (
        <>
          <Glyph kind="web" />
          <span className="name">Search Google for &ldquo;{row.query}&rdquo;</span>
        </>
      );
    case "clip":
      return (
        <>
          <Glyph kind="clip" />
          <span className="name clip">{row.entry.text.slice(0, 120)}</span>
          <span className="sub">copy</span>
        </>
      );
    case "entry":
      return (
        <>
          {row.entry.kind === "app" ? <AppIcon path={row.entry.path} /> : <Glyph kind={row.entry.kind} />}
          <span className="name">{row.entry.name}</span>
          {row.entry.favorite && <span className="sr-only">, pinned</span>}
          <span className="sub path">{row.entry.kind === "app" ? "App" : row.entry.path}</span>
          <PinButton pinned={row.entry.favorite} onToggle={() => onTogglePin(row.entry.path)} />
        </>
      );
    case "favorite":
      return (
        <>
          {row.fav.kind === "app" && !row.fav.missing ? (
            <AppIcon path={row.fav.path} />
          ) : (
            <Glyph kind={row.fav.kind} />
          )}
          <span className={`name ${row.fav.missing ? "missing" : ""}`}>{row.fav.name}</span>
          <span className="sub path">
            {row.fav.missing ? "Not found" : row.fav.kind === "app" ? "App" : row.fav.path}
          </span>
          <PinButton pinned onToggle={() => onTogglePin(row.fav.path)} />
        </>
      );
    case "reindex":
      return (
        <>
          <Glyph kind="reindex" />
          <span className="name">Rebuild index</span>
          <span className="sub">{indexing ? "running…" : "Ctrl+R · picks up new apps & folders"}</span>
        </>
      );
    case "power": {
      const text = POWER_TEXT[row.action];
      const isArmed = armed === row.action;
      return (
        <>
          <Glyph kind={row.action} />
          <span className="name">{isArmed ? text.ask : text.label}</span>
          <span className="sub">
            {isArmed ? "Enter to confirm · type or Esc to cancel" : "Enter, then Enter again to confirm"}
          </span>
        </>
      );
    }
    case "hint":
      return <span className="name">{row.text}</span>;
  }
}
