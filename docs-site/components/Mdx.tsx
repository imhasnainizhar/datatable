import Link from "next/link";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { isValidElement } from "react";
import { createHighlighter, type Highlighter } from "shiki";
import { slugify } from "@/lib/content";
import { CopyButton } from "./CopyButton";

// ---------- Headings with anchor links ----------

function textOf(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(textOf).join("");
  if (isValidElement(node)) return textOf((node.props as { children?: ReactNode }).children);
  return "";
}

function heading(Tag: "h2" | "h3") {
  return function Heading({ children }: { children?: ReactNode }) {
    const id = slugify(textOf(children));
    return (
      <Tag id={id}>
        {children}
        <a className="anchor" href={`#${id}`} aria-label="Link to this section">
          #
        </a>
      </Tag>
    );
  };
}

// ---------- Links: internal ones go through next/link (basePath aware) ----------

function SmartLink({ href = "", ...props }: ComponentProps<"a">) {
  if (href.startsWith("/")) return <Link href={href} {...props} />;
  return <a href={href} {...props} />;
}

// ---------- Code blocks ----------
//
// Conventions in fenced code:
//   ```rust          compiled and run by `npm run check-samples`
//   ```rust-panic    expected to panic when run
//   ```rust-nocheck  shown, never compiled
//   ```rust-sig      a signature line, rendered compactly
//   first line `// file: src/main.rs` (or `# file: ...`) becomes the block title
//   Rust lines starting with "# " are compiled but hidden, as in rustdoc

let highlighter: Promise<Highlighter> | null = null;
function getHighlighter() {
  highlighter ??= createHighlighter({ themes: ["github-light", "github-dark"], langs: ["rust", "toml", "bash"] });
  return highlighter;
}

const LABELS: Record<string, string> = { rust: "Rust", toml: "TOML", bash: "Terminal", text: "Output" };

async function CodeBlock({ children }: { children?: ReactNode }) {
  const code = children as ReactElement<{ className?: string; children?: string }>;
  const raw = String(code?.props?.children ?? "").replace(/\n$/, "");
  const tag = (code?.props?.className ?? "language-text").replace("language-", "");
  const [lang, variant] = tag.split("-") as [string, string | undefined];

  let lines = raw.split("\n");
  let title: string | undefined;
  const fileLine = /^(?:\/\/|#) file: (.+)$/.exec(lines[0] ?? "");
  if (fileLine) {
    title = fileLine[1];
    lines = lines.slice(1);
  }
  if (lang === "rust") lines = lines.filter((l) => !(l.startsWith("# ") || l === "#"));
  const text = lines.join("\n");

  const hl = await getHighlighter();
  const known = ["rust", "toml", "bash"].includes(lang);
  const html = hl.codeToHtml(text, {
    lang: known ? lang : "text",
    themes: { light: "github-light", dark: "github-dark" },
    defaultColor: false,
  });

  if (variant === "sig") {
    return <div dangerouslySetInnerHTML={{ __html: html.replace('class="shiki', 'class="sig shiki') }} />;
  }
  return (
    <div className="code-block" data-lang={lang}>
      <div className="code-header">
        <span className="code-title">{title ?? LABELS[lang] ?? lang}</span>
        <CopyButton text={text} />
      </div>
      <div dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}

// ---------- Layout helpers used directly in MDX ----------

function Callout({ type = "note", title, children }: { type?: "note" | "tip" | "warning"; title?: string; children?: ReactNode }) {
  return (
    <div className={`callout ${type}`}>
      {title && <span className="callout-title">{title}</span>}
      {children}
    </div>
  );
}

function Cards({ children }: { children?: ReactNode }) {
  return <div className="cards">{children}</div>;
}

function Card({ href, title, children }: { href: string; title: string; children?: ReactNode }) {
  return (
    <Link className="card" href={href}>
      <span className="card-title">{title}</span>
      <span className="card-text">{children}</span>
    </Link>
  );
}

function Buttons({ children }: { children?: ReactNode }) {
  return <div className="button-row">{children}</div>;
}

function Button({ href, primary, children }: { href: string; primary?: boolean; children?: ReactNode }) {
  return (
    <Link className={primary ? "button primary" : "button"} href={href}>
      {children}
    </Link>
  );
}

function Badges({ items }: { items: string[] }) {
  return (
    <div className="badges">
      {items.map((b) => (
        <span key={b} className={b.startsWith("O(") ? "badge complexity" : b.startsWith("panics") ? "badge warn" : "badge"}>
          {b}
        </span>
      ))}
    </div>
  );
}

export const mdxComponents = {
  h2: heading("h2"),
  h3: heading("h3"),
  a: SmartLink,
  pre: CodeBlock,
  Callout,
  Cards,
  Card,
  Buttons,
  Button,
  Badges,
};
