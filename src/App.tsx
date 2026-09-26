import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { evaluate, formatResult, looksLikeMath } from "./calculator";
import { RowContent, pinnablePath, rowKey, type Row } from "./rows";
import { firstSelectable, nearestSelectable, step } from "./selection";
import type { ClipEntry, FavoriteView, SearchResult, Status } from "./types";

const EMPTY_TIPS = "Search apps & files  ·  math for calc  ·  g: web  ·  cb clipboard  ·  Ctrl+R reindex";

// Typing any of these (from 3 characters on) offers the rebuild action.
const REINDEX_WORDS = ["reindex", "refresh", "update"];

function looksLikeReindex(q: string): boolean {
  const lower = q.toLowerCase();
  return lower.length >= 3 && REINDEX_WORDS.some((w) => w.startsWith(lower));
}

export default function App() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [sel, setSel] = useState(0);
  const [status, setStatus] = useState<Status | null>(null);
  const [flash, setFlash] = useState<string | null>(null);
  // Bumped to re-run the current query without the user typing, e.g. after a
  // pin changes what the list should show.
  const [refreshKey, setRefreshKey] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const seqRef = useRef(0);
  // After a pin, keep the highlight on the entry that was pinned, wherever the
  // re-ranked list moved it; a new query starts at the top. Set just before
  // bumping refreshKey, read once by the effect.
  const followRef = useRef<{ path: string } | null>(null);
  // A rebuild the user asked for reports when it lands; one the backend started
  // on its own (a stale index refreshing on open) finishes quietly.
  const askedForRebuild = useRef(false);

  const hide = useCallback(() => {
    invoke("hide_window").catch(() => {});
  }, []);

  // Recompute rows whenever the query changes (or a refresh is asked for).
  useEffect(() => {
    const seq = ++seqRef.current;
    const q = query.trim();
    const follow = followRef.current;
    followRef.current = null;
    const show = (next: Row[]) => {
      setRows(next);
      if (!follow) {
        setSel(firstSelectable(next));
        return;
      }
      // Pinning re-ranks the list, so find the entry again by path. If it
      // left the list (unpinned from favorites), take the nearest neighbour.
      const at = next.findIndex((r) => pinnablePath(r) === follow.path);
      setSel((s) => (at >= 0 ? at : nearestSelectable(next, s)));
    };

    // Empty box: the pinned entries, ready for arrow + Enter.
    if (!q) {
      invoke<FavoriteView[]>("list_favorites")
        .then((favs) => {
          if (seqRef.current !== seq) return;
          show(
            favs.length
              ? favs.map((fav) => ({ type: "favorite" as const, fav }))
              : [
                  { type: "hint", text: "No favorites yet. Select a result and press Ctrl+D to pin it here." },
                  { type: "hint", text: EMPTY_TIPS },
                ]
          );
        })
        .catch(() => {
          if (seqRef.current !== seq) return;
          show([
            { type: "hint", text: "Couldn't load favorites. Search still works." },
            { type: "hint", text: EMPTY_TIPS },
          ]);
        });
      return;
    }

    // g: web search
    if (/^g:\s*/i.test(q)) {
      const term = q.replace(/^g:\s*/i, "");
      show(term ? [{ type: "web", query: term }] : [{ type: "hint", text: "Keep typing to search Google…" }]);
      return;
    }

    // cb → clipboard history
    if (q === "cb" || q.startsWith("cb ")) {
      const filter = q.slice(2).trim().toLowerCase();
      invoke<ClipEntry[]>("clipboard_history")
        .then((items) => {
          if (seqRef.current !== seq) return;
          const filtered = filter
            ? items.filter((it) => it.text.toLowerCase().includes(filter))
            : items;
          show(
            filtered.length
              ? filtered.slice(0, 30).map((entry) => ({ type: "clip" as const, entry }))
              : [{ type: "hint", text: "Clipboard history is empty. Copy some text and it shows up here." }]
          );
        })
        .catch(() => {
          if (seqRef.current !== seq) return;
          show([{ type: "hint", text: "Couldn't read clipboard history." }]);
        });
      return;
    }

    // inline calculator
    if (looksLikeMath(q)) {
      const val = evaluate(q);
      if (val !== null) {
        show([{ type: "calc", expr: q, result: formatResult(val) }]);
        return;
      }
    }

    // "reindex" / "refresh" / "update" offer the rebuild action on top of the
    // normal results, so a file that happens to match is still reachable.
    const head: Row[] = looksLikeReindex(q) ? [{ type: "reindex" }] : [];
    if (head.length) show(head);

    // app / file / folder search (debounced lightly)
    const t = setTimeout(() => {
      invoke<SearchResult[]>("search", { query: q })
        .then((res) => {
          if (seqRef.current !== seq) return;
          const tail: Row[] = res.length
            ? res.map((entry) => ({ type: "entry" as const, entry }))
            : head.length
              ? []
              : [
                  { type: "hint", text: "No matches in your apps, files or folders." },
                  { type: "web", query: q },
                ];
          show([...head, ...tail]);
        })
        .catch(() => {
          if (seqRef.current !== seq) return;
          show([
            ...head,
            { type: "hint", text: "Search didn't answer. Ctrl+R rebuilds the index." },
            { type: "web", query: q },
          ]);
        });
    }, 40);
    return () => clearTimeout(t);
  }, [query, refreshKey]);

  // Rebuild the index now. The window stays open so the footer can report
  // progress — this is the one action whose result the user waits for.
  const reindex = useCallback(async () => {
    askedForRebuild.current = true;
    const started = await invoke<boolean>("reindex").catch(() => false);
    setQuery("");
    setFlash(started ? "Rebuilding index…" : "Already rebuilding…");
    invoke<Status>("get_status").then(setStatus).catch(() => {});
  }, []);

  const togglePin = useCallback(async (path: string) => {
    try {
      const pinned = await invoke<boolean>("toggle_favorite", { path });
      setFlash(pinned ? "Pinned" : "Unpinned");
      followRef.current = { path };
      setRefreshKey((k) => k + 1);
    } catch {
      setFlash("Couldn't save favorites");
    }
  }, []);

  // The index can be a little behind the disk (an app uninstalled since the
  // last rebuild), so opening can fail; say so instead of silently staying put.
  const openPath = useCallback(
    async (path: string) => {
      try {
        await invoke("open_entry", { path });
        hide();
      } catch {
        setFlash("Couldn't open it. Ctrl+R rebuilds the index");
      }
    },
    [hide]
  );

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
          await openPath(row.entry.path);
          break;
        case "favorite":
          if (row.fav.missing) {
            setFlash("Not found anymore. Ctrl+D unpins it");
          } else {
            await openPath(row.fav.path);
          }
          break;
        case "reindex":
          await reindex();
          break;
        case "hint":
          break;
      }
    },
    [hide, reindex, openPath]
  );

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "r") {
        e.preventDefault();
        reindex();
      } else if (mod && e.key.toLowerCase() === "d") {
        e.preventDefault();
        const path = pinnablePath(rows[sel]);
        if (path) togglePin(path);
        else setFlash("Only apps, files and folders can be pinned");
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSel((s) => step(rows, s, 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSel((s) => step(rows, s, -1));
      } else if (e.key === "Enter") {
        e.preventDefault();
        activate(rows[sel]);
      }
    },
    [rows, sel, activate, reindex, togglePin]
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
      // The box is often already empty, so the query alone wouldn't re-run
      // the effect; force it so the favorites list is current on every open.
      setRefreshKey((k) => k + 1);
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

  const showingFavorites = rows.some((r) => r.type === "favorite");
  const activeId = sel >= 0 ? `lukfor-row-${sel}` : undefined;

  return (
    <div className="backdrop" onMouseDown={onBackdropClick}>
      <div className="frame">
        <div className="panel panel-in" ref={panelRef}>
          <div className="inputWrap">
            <input
              ref={inputRef}
              autoFocus
              spellCheck={false}
              placeholder="Search Lukfor…"
              aria-label="Search apps, files and folders"
              role="combobox"
              aria-expanded={rows.length > 0}
              aria-controls="lukfor-results"
              aria-activedescendant={activeId}
              aria-autocomplete="list"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={onKeyDown}
            />
            <span className="flash" role="status" aria-live="polite">
              {flash ?? ""}
            </span>
          </div>
          {showingFavorites && (
            <div className="section" aria-hidden="true">
              Favorites
            </div>
          )}
          <div
            className="results"
            id="lukfor-results"
            role="listbox"
            aria-label={showingFavorites ? "Favorites" : "Results"}
          >
            {rows.map((row, i) => {
              const actionable = row.type !== "hint";
              return (
                <div
                  key={rowKey(row, i)}
                  id={`lukfor-row-${i}`}
                  data-row={i}
                  role="option"
                  aria-selected={i === sel}
                  aria-disabled={actionable ? undefined : true}
                  className={`row ${i === sel ? "selected" : ""} ${actionable ? "" : "hint"}`}
                  onMouseEnter={actionable ? () => setSel(i) : undefined}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    if (actionable) activate(row);
                  }}
                >
                  <RowContent row={row} indexing={!!status?.indexing} onTogglePin={togglePin} />
                </div>
              );
            })}
          </div>
          <div className="footer">
            <span>{status ? status.hotkey : ""}</span>
            <span>
              {status
                ? `${status.indexed.toLocaleString()} items${status.indexing ? " · indexing" : ""}`
                : ""}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
