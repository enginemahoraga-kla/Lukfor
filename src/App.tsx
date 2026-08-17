import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { evaluate, formatResult, looksLikeMath } from "./calculator";
import type { ClipEntry, SearchResult, Status } from "./types";

type Row =
  | { type: "calc"; expr: string; result: string }
  | { type: "web"; query: string }
  | { type: "clip"; entry: ClipEntry }
  | { type: "entry"; entry: SearchResult }
  | { type: "reindex" }
  | { type: "hint"; text: string };

// Typing any of these (from 3 characters on) offers the rebuild action.
const REINDEX_WORDS = ["reindex", "refresh", "update"];

function looksLikeReindex(q: string): boolean {
  const lower = q.toLowerCase();
  return lower.length >= 3 && REINDEX_WORDS.some((w) => w.startsWith(lower));
}

const KIND_ICON: Record<string, string> = {
  app: "▸", // ▸
  folder: "\u{1F4C1}",
  file: "\u{1F4C4}",
};

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
  if (!src) return <span className="icon">{KIND_ICON.app}</span>;
  return (
    <span className="icon">
      <img className="appicon" src={src} alt="" draggable={false} />
    </span>
  );
}

function rowKey(r: Row, i: number): string {
  return `${r.type}-${i}`;
}

export default function App() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [sel, setSel] = useState(0);
  const [status, setStatus] = useState<Status | null>(null);
  const [flash, setFlash] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const seqRef = useRef(0);
  // A rebuild the user asked for reports when it lands; one the backend started
  // on its own (a stale index refreshing on open) finishes quietly.
  const askedForRebuild = useRef(false);

  const hide = useCallback(() => {
    invoke("hide_window").catch(() => {});
  }, []);

  // Recompute rows whenever the query changes.
  useEffect(() => {
    const seq = ++seqRef.current;
    const q = query.trim();

    if (!q) {
      setRows([{ type: "hint", text: "Search apps & files  ·  math for calc  ·  g: web  ·  cb clipboard  ·  Ctrl+R reindex" }]);
      setSel(0);
      return;
    }

    // g: web search
    if (/^g:\s*/i.test(q)) {
      const term = q.replace(/^g:\s*/i, "");
      setRows(term ? [{ type: "web", query: term }] : [{ type: "hint", text: "Keep typing to search Google…" }]);
      setSel(0);
      return;
    }

    // cb → clipboard history
    if (q === "cb" || q.startsWith("cb ")) {
      const filter = q.slice(2).trim().toLowerCase();
      invoke<ClipEntry[]>("clipboard_history").then((items) => {
        if (seqRef.current !== seq) return;
        const filtered = filter
          ? items.filter((it) => it.text.toLowerCase().includes(filter))
          : items;
        setRows(
          filtered.length
            ? filtered.slice(0, 30).map((entry) => ({ type: "clip" as const, entry }))
            : [{ type: "hint", text: "Clipboard history is empty" }]
        );
        setSel(0);
      });
      return;
    }

    // inline calculator
    if (looksLikeMath(q)) {
      const val = evaluate(q);
      if (val !== null) {
        setRows([{ type: "calc", expr: q, result: formatResult(val) }]);
        setSel(0);
        return;
      }
    }

    // "reindex" / "refresh" / "update" offer the rebuild action on top of the
    // normal results, so a file that happens to match is still reachable.
    const head: Row[] = looksLikeReindex(q) ? [{ type: "reindex" }] : [];
    if (head.length) {
      setRows(head);
      setSel(0);
    }

    // app / file / folder search (debounced lightly)
    const t = setTimeout(() => {
      invoke<SearchResult[]>("search", { query: q }).then((res) => {
        if (seqRef.current !== seq) return;
        const tail: Row[] = res.length
          ? res.map((entry) => ({ type: "entry" as const, entry }))
          : head.length
            ? []
            : [{ type: "hint", text: "No results — press Enter to search Google" }];
        setRows([...head, ...tail]);
        setSel(0);
      });
    }, 40);
    return () => clearTimeout(t);
  }, [query]);

  // Rebuild the index now. The window stays open so the footer can report
  // progress — this is the one action whose result the user waits for.
  const reindex = useCallback(async () => {
    askedForRebuild.current = true;
    const started = await invoke<boolean>("reindex").catch(() => false);
    setQuery("");
    setFlash(started ? "Rebuilding index…" : "Already rebuilding…");
    invoke<Status>("get_status").then(setStatus).catch(() => {});
  }, []);

  const activate = useCallback(
    async (row: Row | undefined) => {
      if (!row) return;
      switch (row.type) {
        case "calc":
          await invoke("copy_text", { text: row.result });
          setFlash("Copied result");
          setTimeout(hide, 350);
          break;
        case "web":
          await invoke("open_url", {
            url: `https://www.google.com/search?q=${encodeURIComponent(row.query)}`,
          });
          hide();
          break;
        case "clip":
          await invoke("copy_text", { text: row.entry.text });
          setFlash("Copied");
          setTimeout(hide, 350);
          break;
        case "entry":
          await invoke("open_entry", { path: row.entry.path });
          hide();
          break;
        case "reindex":
          await reindex();
          break;
        case "hint":
          if (query.trim() && !query.trim().startsWith("cb")) {
            await invoke("open_url", {
              url: `https://www.google.com/search?q=${encodeURIComponent(query.trim())}`,
            });
            hide();
          }
          break;
      }
    },
    [hide, query, reindex]
  );

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key.toLowerCase() === "r" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        reindex();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSel((s) => Math.min(s + 1, rows.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSel((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        activate(rows[sel]);
      }
    },
    [rows, sel, activate, reindex]
  );

  // While a rebuild runs, keep the footer count live and report when it lands.
  useEffect(() => {
    if (!status?.indexing) return;
    const id = setInterval(() => {
      invoke<Status>("get_status")
        .then((next) => {
          setStatus(next);
          if (!next.indexing && askedForRebuild.current) {
            askedForRebuild.current = false;
            setFlash(`Index updated · ${next.indexed.toLocaleString()} items`);
          }
        })
        .catch(() => {});
    }, 700);
    return () => clearInterval(id);
  }, [status?.indexing]);

  // Esc must always close, even if focus has drifted off the input (e.g.
  // Tab moved it to the body), so listen at the window level rather than
  // only on the input's onKeyDown.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        hide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [hide]);

  // Window shown → reset + focus + replay fade-in.
  useEffect(() => {
    const un = listen("lukfor://shown", () => {
      setQuery("");
      setSel(0);
      setFlash(null);
      invoke<Status>("get_status").then(setStatus).catch(() => {});
      const el = panelRef.current;
      if (el) {
        el.classList.remove("panel-in");
        void el.offsetWidth; // restart animation
        el.classList.add("panel-in");
      }
      requestAnimationFrame(() => inputRef.current?.focus());
    });
    invoke<Status>("get_status").then(setStatus).catch(() => {});
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Clear a finished flash on its own; "Rebuilding…" stays up until it lands.
  useEffect(() => {
    if (!flash || status?.indexing) return;
    const id = setTimeout(() => setFlash(null), 2500);
    return () => clearTimeout(id);
  }, [flash, status?.indexing]);

  // Click outside the panel hides the window.
  const onBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) hide();
    },
    [hide]
  );

  // Keep selection visible.
  useEffect(() => {
    document
      .querySelector(`[data-row="${sel}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  return (
    <div className="backdrop" onMouseDown={onBackdropClick}>
      <div className="panel panel-in" ref={panelRef}>
        <div className="inputWrap">
          <span className="prompt">&#9670;</span>
          <input
            ref={inputRef}
            autoFocus
            spellCheck={false}
            placeholder="Search Lukfor…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
          />
          {flash && <span className="flash">{flash}</span>}
        </div>
        <div className="results">
          {rows.map((row, i) => (
            <div
              key={rowKey(row, i)}
              data-row={i}
              className={`row ${i === sel ? "selected" : ""} ${row.type === "hint" ? "hint" : ""}`}
              onMouseEnter={() => setSel(i)}
              onMouseDown={(e) => {
                e.preventDefault();
                activate(row);
              }}
            >
              {row.type === "calc" && (
                <>
                  <span className="icon">=</span>
                  <span className="name mono">{row.result}</span>
                  <span className="sub">Enter to copy</span>
                </>
              )}
              {row.type === "web" && (
                <>
                  <span className="icon">&#127760;</span>
                  <span className="name">Search Google for &ldquo;{row.query}&rdquo;</span>
                </>
              )}
              {row.type === "clip" && (
                <>
                  <span className="icon">&#128203;</span>
                  <span className="name clip">{row.entry.text.slice(0, 120)}</span>
                  <span className="sub">copy</span>
                </>
              )}
              {row.type === "entry" && (
                <>
                  {row.entry.kind === "app" ? (
                    <AppIcon path={row.entry.path} />
                  ) : (
                    <span className="icon">{KIND_ICON[row.entry.kind]}</span>
                  )}
                  <span className="name">{row.entry.name}</span>
                  <span className="sub path">
                    {row.entry.kind === "app" ? "App" : row.entry.path}
                  </span>
                </>
              )}
              {row.type === "reindex" && (
                <>
                  <span className="icon">&#8635;</span>
                  <span className="name">Rebuild index</span>
                  <span className="sub">
                    {status?.indexing ? "running…" : "Ctrl+R · picks up new apps & folders"}
                  </span>
                </>
              )}
              {row.type === "hint" && <span className="name">{row.text}</span>}
            </div>
          ))}
        </div>
        <div className="footer">
          <span>{status ? status.hotkey : ""}</span>
          <span>
            {status
              ? `${status.indexed.toLocaleString()} items${status.indexing ? " (indexing…)" : ""}`
              : ""}
          </span>
        </div>
      </div>
    </div>
  );
}
