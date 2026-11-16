"use client";

import { useEffect, useState } from "react";
import type { Heading } from "@/lib/content";

export function Toc({ headings }: { headings: Heading[] }) {
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    const elements = headings
      .map((h) => document.getElementById(h.id))
      .filter((el): el is HTMLElement => el !== null);
    const onScroll = () => {
      let current: string | null = elements[0]?.id ?? null;
      for (const el of elements) {
        if (el.getBoundingClientRect().top < 120) current = el.id;
        else break;
      }
      setActive(current);
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [headings]);

  if (headings.length < 2) return null;
  return (
    <div className="toc-inner">
      <p className="toc-title">On this page</p>
      <ul>
        {headings.map((h) => (
          <li key={h.id} className={`toc-l${h.level}`}>
            <a href={`#${h.id}`} className={active === h.id ? "active" : undefined}>
              {h.text}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
