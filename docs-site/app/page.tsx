import type { Metadata } from "next";
import Link from "next/link";
import { CopyButton } from "@/components/CopyButton";
import { highlightToHtml } from "@/components/Mdx";
import { GITHUB, VERSION } from "@/lib/nav";

export const metadata: Metadata = {
  title: { absolute: "datatable: the flexible table for Rust" },
  description:
    "One dynamic container for Rust that works as an array, a dictionary, a record or a set. Inspired by Lua tables, built for speed, with no unsafe code.",
};

const INSTALL = "cargo add datatable --git https://github.com/imhasnainizhar/datatable";

const SHOWCASE = `use datatable::{table, Table, Value};

let player = Table::new();
player.set("name", "Ada")?;          // named fields
player.set("level", 7)?;
player.push("sword")?;               // and a sequence
player.push("shield")?;

assert_eq!(player.len(), 2);
assert_eq!(player.field_count(), 4);
assert_eq!(player.get("name"), Value::from("Ada"));

let scores = table![42, 7, 19];
scores.sort()?;
assert_eq!(scores.concat(", ")?, "7, 19, 42");`;

const FEATURES = [
  {
    title: "One type, many shapes",
    text: "Use the same Table as a list, a dictionary, a record or a set, or all of them at once.",
    href: "/getting-started/core-concepts/",
  },
  {
    title: "Sequence library",
    text: "push, pop, insert, remove, concat, sort and move ranges, with 1-based positions inspired by Lua.",
    href: "/guides/sequences/",
  },
  {
    title: "Iteration that bends",
    text: "pairs and ipairs tolerate changes mid-loop, next is O(1), and borrowing iterators never clone.",
    href: "/guides/iteration/",
  },
  {
    title: "Shared and nestable",
    text: "Tables are cheap handles. Nest them, share them, and copy shallowly or deeply, cycles included.",
    href: "/guides/nesting-and-copying/",
  },
  {
    title: "Built for speed",
    text: "O(1) length, an array part for integer keys, an ordered hash part, and lookups that never allocate.",
    href: "/guides/performance/",
  },
  {
    title: "No surprises",
    text: "Typed errors instead of panics, a deterministic traversal order, and no unsafe code anywhere.",
    href: "/guides/error-handling/",
  },
];

const STATS = [
  { value: "1.1 ns", label: "to overwrite a sequence slot" },
  { value: "0", label: "lines of unsafe code" },
  { value: "58", label: "doc examples compiled and run" },
];

const FOOTER = [
  {
    title: "Get started",
    links: [
      ["Installation", "/getting-started/installation/"],
      ["Quick Start", "/getting-started/quick-start/"],
      ["Core Concepts", "/getting-started/core-concepts/"],
    ],
  },
  {
    title: "Guides",
    links: [
      ["Sequences", "/guides/sequences/"],
      ["Iteration", "/guides/iteration/"],
      ["Performance", "/guides/performance/"],
      ["Coming from Lua", "/guides/coming-from-lua/"],
    ],
  },
  {
    title: "Reference",
    links: [
      ["API Overview", "/api/"],
      ["Table", "/api/table/"],
      ["Value", "/api/value/"],
      ["What's New", "/whats-new/"],
    ],
  },
  {
    title: "Project",
    links: [
      ["GitHub", GITHUB],
      ["Issues", `${GITHUB}/issues`],
      ["License", `${GITHUB}/blob/main/LICENSE`],
    ],
  },
];

function FooterLink({ href, children }: { href: string; children: string }) {
  return href.startsWith("/") ? <Link href={href}>{children}</Link> : <a href={href}>{children}</a>;
}

export default async function Home() {
  const code = await highlightToHtml(SHOWCASE, "rust");

  return (
    <main className="landing" id="content">
      <section className="hero">
        <div className="hero-grid-bg" aria-hidden="true" />
        <Link className="hero-pill" href="/whats-new/">
          <span className="hero-pill-tag">New</span> datatable {VERSION} is out <span aria-hidden="true">→</span>
        </Link>
        <h1 className="hero-title">
          The flexible table
          <br />
          <span className="hero-accent">for Rust</span>
        </h1>
        <p className="hero-sub">
          One dynamic container that works as an array, a dictionary, a record or a set. Inspired by Lua tables,
          built for speed, with no <code>unsafe</code> code.
        </p>
        <div className="hero-actions">
          <Link className="cta cta-primary" href="/getting-started/installation/">
            Get Started
          </Link>
          <Link className="cta cta-secondary" href="/getting-started/quick-start/">
            Take the Quick Start
          </Link>
        </div>
        <div className="hero-install">
          <span className="hero-install-prompt" aria-hidden="true">
            $
          </span>
          <code>{INSTALL}</code>
          <CopyButton text={INSTALL} />
        </div>
      </section>

      <section className="showcase">
        <div className="showcase-text">
          <p className="section-eyebrow">One structure</p>
          <h2>Sequences and fields, side by side</h2>
          <p>
            A table keeps integer keys in an array and everything else in an insertion-ordered hash part. You never
            choose between a <code>Vec</code> and a <code>HashMap</code>: write fields, push items, nest tables, and
            read them back in a predictable order.
          </p>
          <Link className="text-link" href="/getting-started/core-concepts/">
            Learn the core concepts →
          </Link>
        </div>
        <div className="showcase-code code-block" data-lang="rust">
          <div className="code-header">
            <span className="code-title">src/main.rs</span>
            <CopyButton text={SHOWCASE} />
          </div>
          <div dangerouslySetInnerHTML={{ __html: code }} />
        </div>
      </section>

      <section className="features">
        <p className="section-eyebrow">Features</p>
        <h2>What&apos;s in datatable?</h2>
        <div className="feature-grid">
          {FEATURES.map((f) => (
            <Link key={f.title} className="feature" href={f.href}>
              <span className="feature-title">{f.title}</span>
              <span className="feature-text">{f.text}</span>
            </Link>
          ))}
        </div>
      </section>

      <section className="stats">
        {STATS.map((s) => (
          <div key={s.label} className="stat">
            <strong>{s.value}</strong>
            <span>{s.label}</span>
          </div>
        ))}
      </section>

      <section className="final-cta">
        <h2>Start building with tables</h2>
        <p>Install the crate and write your first table in about five minutes.</p>
        <div className="hero-actions">
          <Link className="cta cta-primary" href="/getting-started/installation/">
            Get Started
          </Link>
          <Link className="cta cta-secondary" href="/api/">
            Browse the API
          </Link>
        </div>
      </section>

      <footer className="site-footer">
        <div className="site-footer-inner">
          <div className="footer-brand">
            <span className="brand-name">datatable</span>
            <span>A flexible table type for Rust. Apache-2.0.</span>
          </div>
          {FOOTER.map((col) => (
            <div key={col.title} className="footer-col">
              <p>{col.title}</p>
              <ul>
                {col.links.map(([label, href]) => (
                  <li key={label}>
                    <FooterLink href={href}>{label}</FooterLink>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </footer>
    </main>
  );
}
