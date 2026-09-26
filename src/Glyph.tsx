// Monochrome row glyphs, drawn for Lukfor rather than taken from an icon set:
// square caps, mitred joins and the same clipped corner the panel uses, so the
// marks read as one family with the rest of the UI. They take the row's text
// color, which keeps them legible on the solid selection block too.

export type GlyphKind = "app" | "file" | "folder" | "web" | "clip" | "calc" | "reindex";

const PATHS: Record<GlyphKind, string> = {
  // Four tiles, the common "apps" mark, for an app with no icon of its own.
  // (A single cut-corner tile read as a document at 16px.)
  app: "M2.5 2.5h4.25v4.25H2.5z M9.25 2.5h4.25v4.25H9.25z M2.5 9.25h4.25v4.25H2.5z M9.25 9.25h4.25v4.25H9.25z",
  // Folder with an angled tab.
  folder: "M1.75 3.25h4.75l1.5 1.75h6.25v7.75H1.75z",
  // Page with the clipped corner folded in.
  file: "M3.25 1.75h6.25l3.25 3.25v9.25H3.25z M9.5 1.75v3.25h3.25",
  // Leaving Lukfor for the browser: an arrow out of a box.
  web: "M9 2.25h4.75V7 M13.75 2.25 7.5 8.5 M11.75 9.5v4.25h-9.5v-9.5H6.5",
  clip: "M5 3.25H3v10.5h10V3.25h-2 M5.75 1.75h4.5v2.75h-4.5z",
  calc: "M3 6h10 M3 10h10",
  reindex:
    "M13 6.25A5.25 5.25 0 0 0 3.25 5.5 M3 9.75A5.25 5.25 0 0 0 12.75 10.5 M13.25 2v4.25H9 M2.75 14V9.75H7",
};

export function Glyph({ kind }: { kind: GlyphKind }) {
  return (
    <span className="icon" aria-hidden="true">
      <svg viewBox="0 0 16 16" width="16" height="16">
        <path d={PATHS[kind]} />
      </svg>
    </span>
  );
}

// Filled when pinned, outlined when not: the shape alone carries the state,
// so it survives any color the row gives it.
export function StarGlyph({ filled }: { filled: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <path
        className={filled ? "star filled" : "star"}
        d="M8 1.6 9.95 5.95 14.4 6.35 11 9.3 12.05 13.9 8 11.45 3.95 13.9 5 9.3 1.6 6.35 6.05 5.95z"
      />
    </svg>
  );
}
