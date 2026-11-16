import Link from "next/link";
import { GITHUB, NAV, PAGES, VERSION, findPage, pageHref } from "@/lib/nav";
import { headings, loadPage } from "@/lib/content";
import { Toc } from "./Toc";

export async function DocPage({ slug }: { slug: string }) {
  const page = findPage(slug)!;
  const { default: Content } = await loadPage(slug);
  const index = PAGES.findIndex((p) => p.slug === slug);
  const prev = PAGES[index - 1];
  const next = PAGES[index + 1];
  const file = slug === "" ? "index" : slug === "api" ? "api/index" : slug;

  return (
    <div className="layout">
      <aside className="sidebar" id="sidebar" aria-label="Documentation">
        <nav>
          {NAV.map((group) => (
            <div className="nav-group" key={group.section || "top"}>
              {group.section && <p className="nav-heading">{group.section}</p>}
              <ul>
                {group.pages.map((p) => (
                  <li key={p.slug}>
                    <Link
                      href={pageHref(p.slug)}
                      className={p.slug === slug ? "active" : undefined}
                      aria-current={p.slug === slug ? "page" : undefined}
                    >
                      {p.nav ?? p.title}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </aside>
      <main className="content" id="content">
        <article className={slug.startsWith("api/") ? "prose api" : "prose"}>
          {page.section && <p className="eyebrow">{page.section}</p>}
          <h1>{page.heading ?? page.title}</h1>
          <p className="lead">{page.description}</p>
          <Content />
          <nav className="pager" aria-label="Pagination">
            {prev ? (
              <Link className="pager-prev" href={pageHref(prev.slug)}>
                <span className="pager-label">Previous</span>
                <span className="pager-title">{prev.nav ?? prev.title}</span>
              </Link>
            ) : (
              <span />
            )}
            {next && (
              <Link className="pager-next" href={pageHref(next.slug)}>
                <span className="pager-label">Next</span>
                <span className="pager-title">{next.nav ?? next.title}</span>
              </Link>
            )}
          </nav>
          <footer className="page-footer">
            <a href={`${GITHUB}/edit/main/docs-site/content/${file}.mdx`}>Edit this page on GitHub</a>
            <span>datatable {VERSION} · Apache-2.0</span>
          </footer>
        </article>
      </main>
      <aside className="toc" aria-label="On this page">
        <Toc headings={headings(slug)} />
      </aside>
    </div>
  );
}
