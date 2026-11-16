"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { SearchEntry } from "@/lib/content";
import { pageHref } from "@/lib/nav";

type Result = { title: string; path: string; href: string; text: string; page: boolean };

function score(r: Result, terms: string[], query: string): number {
  if (!terms.every((t) => r.text.includes(t))) return 0;
  const title = r.title.toLowerCase();
  let s = 1;
  if (title === query) s += 200;
  else if (title.startsWith(query)) s += 120;
  else if (title.includes(query)) s += 80;
  for (const t of terms) if (title.includes(t)) s += 20;
  if (r.page) s += 15;
  return s;
}

export function Search({ index }: { index: SearchEntry[] }) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  const all = useMemo<Result[]>(
    () =>
      index.flatMap((p) => [
        { title: p.t, path: p.s || "Docs", href: pageHref(p.u), text: `${p.t} ${p.d}`.toLowerCase(), page: true },
        ...p.h.map(([text, id]) => ({
          title: text,
          path: p.s ? `${p.s} › ${p.t}` : p.t,
          href: `${pageHref(p.u)}#${id}`,
          text: `${text} ${p.t}`.toLowerCase(),
          page: false,
        })),
      ]),
    [index],
  );

  const q = query.trim().toLowerCase();
  const terms = q.split(/\s+/).filter(Boolean);
  const results = terms.length
    ? all
        .map((r) => ({ r, s: score(r, terms, q) }))
        .filter((x) => x.s > 0)
        .sort((a, b) => b.s - a.s)
        .slice(0, 12)
        .map((x) => x.r)
    : all.filter((r) => r.page).slice(0, 8);

  const show = useCallback(() => {
    setQuery("");
    setSelected(0);
    setOpen(true);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement as HTMLElement | null;
      const typing = !!el && (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable);
      if ((e.key === "k" || e.key === "K") && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((o) => !o);
      } else if (e.key === "/" && !typing) {
        e.preventDefault();
        show();
      } else if (e.key === "Escape") {
        setOpen(false);
        document.body.classList.remove("nav-open");
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("open-search", show);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("open-search", show);
    };
  }, [show]);

  useEffect(() => {
    if (open) input.current?.focus();
    document.body.style.overflow = open ? "hidden" : "";
  }, [open]);

  const go = (href: string) => {
    setOpen(false);
    router.push(href);
  };

  if (!open) return null;
  return (
    <div
      className="search-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Search documentation"
      onClick={(e) => e.target === e.currentTarget && setOpen(false)}
    >
      <div className="search-panel">
        <div className="search-field">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="11" cy="11" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" />
            <path stroke="currentColor" strokeWidth="2" strokeLinecap="round" d="m20 20-4.2-4.2" />
          </svg>
          <input
            ref={input}
            type="search"
            placeholder="Search pages, methods and sections"
            autoComplete="off"
            spellCheck={false}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelected(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelected((s) => (s + 1) % Math.max(results.length, 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelected((s) => (s - 1 + results.length) % Math.max(results.length, 1));
              } else if (e.key === "Enter" && results[selected]) {
                e.preventDefault();
                go(results[selected].href);
              }
            }}
          />
          <kbd>Esc</kbd>
        </div>
        <ul className="search-results" role="listbox">
          {results.length === 0 && <li className="search-empty">No results for “{query.trim()}”</li>}
          {!terms.length && <li className="search-group">Pages</li>}
          {results.map((r, i) => (
            <li key={r.href}>
              <a
                role="option"
                aria-selected={i === selected}
                href={r.href}
                onMouseEnter={() => setSelected(i)}
                onClick={(e) => {
                  e.preventDefault();
                  go(r.href);
                }}
              >
                <span className="result-title">{r.title}</span>
                <span className="result-path">{r.path}</span>
              </a>
            </li>
          ))}
        </ul>
        <p className="search-hint">
          <kbd>↑</kbd>
          <kbd>↓</kbd> to navigate <kbd>Enter</kbd> to open
        </p>
      </div>
    </div>
  );
}
