"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { CRATES_IO, GITHUB, VERSION } from "@/lib/nav";

function setNav(open: boolean) {
  document.body.classList.toggle("nav-open", open);
}

export function TopBar() {
  const pathname = usePathname();
  const [mac, setMac] = useState(false);

  useEffect(() => setNav(false), [pathname]);
  useEffect(() => setMac(/Mac|iPhone|iPad/.test(navigator.userAgent)), []);

  const inApi = pathname.startsWith("/api");
  const inNews = pathname.startsWith("/whats-new");

  const toggleTheme = () => {
    const root = document.documentElement;
    const current =
      root.getAttribute("data-theme") ??
      (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
    const next = current === "dark" ? "light" : "dark";
    root.setAttribute("data-theme", next);
    try {
      localStorage.setItem("dt-theme", next);
    } catch {
      /* storage unavailable: the choice lasts for this page only */
    }
  };

  return (
    <>
      <header className="topbar">
        <div className="topbar-inner">
          <button
            hidden={pathname === "/"}
            className="icon-button menu-button"
            type="button"
            aria-label="Open navigation"
            aria-controls="sidebar"
            onClick={() => setNav(!document.body.classList.contains("nav-open"))}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path stroke="currentColor" strokeWidth="2" strokeLinecap="round" d="M4 7h16M4 12h16M4 17h16" />
            </svg>
          </button>
          <Link className="brand" href="/">
            <svg className="logo-mark" viewBox="0 0 32 32" aria-hidden="true">
              <rect x="2" y="2" width="28" height="28" rx="7" className="logo-bg" />
              <rect x="7" y="8" width="18" height="4" rx="1.5" className="logo-head" />
              <rect x="7" y="14.5" width="7.5" height="3.5" rx="1.2" className="logo-cell" />
              <rect x="17.5" y="14.5" width="7.5" height="3.5" rx="1.2" className="logo-cell" />
              <rect x="7" y="20.5" width="7.5" height="3.5" rx="1.2" className="logo-cell" />
              <rect x="17.5" y="20.5" width="7.5" height="3.5" rx="1.2" className="logo-cell" />
            </svg>
            <span className="brand-name">flextable</span>
          </Link>
          <span className="version">v{VERSION}</span>
          <nav className="topnav" aria-label="Primary">
            <Link href="/getting-started/installation/" className={!inApi && !inNews && pathname !== "/" ? "active" : undefined}>
              Docs
            </Link>
            <Link href="/api/" className={inApi ? "active" : undefined}>
              API
            </Link>
            <Link href="/whats-new/" className={inNews ? "active" : undefined}>
              What&apos;s New
            </Link>
          </nav>
          <div className="topbar-actions">
            <button
              className="search-button"
              type="button"
              aria-label="Search documentation"
              onClick={() => window.dispatchEvent(new Event("open-search"))}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="11" cy="11" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" />
                <path stroke="currentColor" strokeWidth="2" strokeLinecap="round" d="m20 20-4.2-4.2" />
              </svg>
              <span className="search-label">Search docs</span>
              <kbd className="search-kbd">{mac ? "⌘K" : "Ctrl K"}</kbd>
            </button>
            <a className="icon-button" href={CRATES_IO} aria-label="flextable on crates.io" title="crates.io">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinejoin="round"
                  d="M12 2.8 20 7.2v9.6L12 21.2 4 16.8V7.2L12 2.8Z"
                />
                <path fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" d="M4 7.2 12 11.6l8-4.4M12 11.6v9.6" />
              </svg>
            </a>
            <a className="icon-button" href={GITHUB} aria-label="flextable on GitHub">
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path
                  fill="currentColor"
                  d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
                />
              </svg>
            </a>
            <button className="icon-button" type="button" aria-label="Toggle dark mode" onClick={toggleTheme}>
              <svg className="icon-sun" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="12" cy="12" r="4.5" fill="none" stroke="currentColor" strokeWidth="1.8" />
                <path
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                  d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"
                />
              </svg>
              <svg className="icon-moon" viewBox="0 0 24 24" aria-hidden="true">
                <path
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinejoin="round"
                  d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"
                />
              </svg>
            </button>
          </div>
        </div>
      </header>
      <div className="scrim" onClick={() => setNav(false)} />
    </>
  );
}
