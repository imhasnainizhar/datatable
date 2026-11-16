import fs from "node:fs";
import path from "node:path";
import type { ComponentType } from "react";
import { PAGES } from "./nav";

type MDXModule = { default: ComponentType };

// Explicit imports keep every page visible to the bundler.
const LOADERS: Record<string, () => Promise<MDXModule>> = {
  "whats-new": () => import("@/content/whats-new.mdx"),
  "getting-started/installation": () => import("@/content/getting-started/installation.mdx"),
  "getting-started/quick-start": () => import("@/content/getting-started/quick-start.mdx"),
  "getting-started/core-concepts": () => import("@/content/getting-started/core-concepts.mdx"),
  "guides/sequences": () => import("@/content/guides/sequences.mdx"),
  "guides/records-and-sets": () => import("@/content/guides/records-and-sets.mdx"),
  "guides/iteration": () => import("@/content/guides/iteration.mdx"),
  "guides/nesting-and-copying": () => import("@/content/guides/nesting-and-copying.mdx"),
  "guides/error-handling": () => import("@/content/guides/error-handling.mdx"),
  "guides/performance": () => import("@/content/guides/performance.mdx"),
  "guides/coming-from-lua": () => import("@/content/guides/coming-from-lua.mdx"),
  api: () => import("@/content/api/index.mdx"),
  "api/table": () => import("@/content/api/table.mdx"),
  "api/table-ref": () => import("@/content/api/table-ref.mdx"),
  "api/value": () => import("@/content/api/value.mdx"),
  "api/key": () => import("@/content/api/key.mdx"),
  "api/error": () => import("@/content/api/error.mdx"),
  "api/table-macro": () => import("@/content/api/table-macro.mdx"),
};

export function loadPage(slug: string): Promise<MDXModule> {
  return LOADERS[slug]();
}

export function sourcePath(slug: string): string {
  const file = slug === "api" ? "api/index" : slug;
  return path.join(process.cwd(), "content", `${file}.mdx`);
}

/** Same rule as the Heading component, so outline links match heading ids. */
export function slugify(text: string): string {
  return (
    text
      .toLowerCase()
      .replace(/[^\w\s-]/g, "")
      .trim()
      .replace(/\s+/g, "-")
      .replace(/^-+|-+$/g, "") || "section"
  );
}

function plain(markdown: string): string {
  return markdown
    .replace(/`([^`]*)`/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[*_]/g, "")
    .trim();
}

export type Heading = { level: 2 | 3; id: string; text: string };

export function headings(slug: string): Heading[] {
  const source = fs.readFileSync(sourcePath(slug), "utf8");
  const out: Heading[] = [];
  let inFence = false;
  for (const line of source.split(/\r?\n/)) {
    if (line.startsWith("```")) inFence = !inFence;
    if (inFence) continue;
    const m = /^(##|###) (.+)$/.exec(line);
    if (m) {
      const text = plain(m[2]);
      out.push({ level: m[1].length as 2 | 3, id: slugify(text), text });
    }
  }
  return out;
}

export type SearchEntry = { t: string; s: string; u: string; d: string; h: [string, string][] };

export function searchIndex(): SearchEntry[] {
  return PAGES.map((p) => ({
    t: p.title,
    s: p.section,
    u: p.slug,
    d: p.description,
    h: headings(p.slug).map((h) => [h.text, h.id] as [string, string]),
  }));
}
