export type DocPage = {
  /** URL path without slashes; "" is the introduction. Also the MDX file path. */
  slug: string;
  title: string;
  /** Shorter label for the sidebar, when it differs from the title. */
  nav?: string;
  heading?: string;
  description: string;
  section: string;
};

export const GITHUB = "https://github.com/imhasnainizhar/datatable";
export const VERSION = "0.1.0";

export const NAV: { section: string; pages: DocPage[] }[] = [
  {
    section: "",
    pages: [
      {
        slug: "",
        title: "Introduction",
        heading: "Welcome to the datatable docs",
        description:
          "datatable gives Rust one dynamic container that works as an array, a dictionary, a record or a set, with an API inspired by Lua tables.",
        section: "",
      },
      {
        slug: "whats-new",
        title: "What's New in 0.1.0",
        nav: "What's New",
        description: "The first release: the table type, its sequence and traversal library, and the performance work behind it.",
        section: "",
      },
    ],
  },
  {
    section: "Getting Started",
    pages: [
      { slug: "getting-started/installation", title: "Installation", description: "Add datatable to a Rust project and check that everything works.", section: "Getting Started" },
      { slug: "getting-started/quick-start", title: "Quick Start", description: "Build a small inventory program and meet the core of the API in about five minutes.", section: "Getting Started" },
      { slug: "getting-started/core-concepts", title: "Core Concepts", description: "Seven rules explain how every table method behaves. Learn them once and the rest of the API is predictable.", section: "Getting Started" },
    ],
  },
  {
    section: "Guides",
    pages: [
      { slug: "guides/sequences", title: "Sequences", description: "Use a table as a list: create, read, grow, shrink, join, sort and move ranges, with 1-based positions.", section: "Guides" },
      { slug: "guides/records-and-sets", title: "Records, Dictionaries and Sets", nav: "Records and Sets", description: "Use string and other keys for named fields, lookups and membership tests.", section: "Guides" },
      { slug: "guides/iteration", title: "Iteration", description: "Four ways to walk a table, what order they use, and which ones let you change the table mid-loop.", section: "Guides" },
      { slug: "guides/nesting-and-copying", title: "Nesting and Copying", description: "Tables are shared handles. Learn how nesting works and how to make shallow and deep copies, cycles included.", section: "Guides" },
      { slug: "guides/error-handling", title: "Error Handling", description: "The five ways a table operation can fail, and how to handle each one.", section: "Guides" },
      { slug: "guides/performance", title: "Performance", description: "How the table is built for speed, benchmark results, and habits that keep your code fast.", section: "Guides" },
      { slug: "guides/coming-from-lua", title: "Coming from Lua", description: "datatable is inspired by Lua tables, not compatible with them. Here is what carries over and what differs.", section: "Guides" },
    ],
  },
  {
    section: "API Reference",
    pages: [
      { slug: "api", title: "API Reference", nav: "Overview", description: "Every public item in the crate, grouped by type.", section: "API Reference" },
      { slug: "api/table", title: "Table", description: "The table type: construction, field access, sequence operations, traversal and copying.", section: "API Reference" },
      { slug: "api/table-ref", title: "TableRef and Iterators", nav: "TableRef and Iterators", description: "The read-only borrowed view, and the Pairs, IPairs and Iter iterators.", section: "API Reference" },
      { slug: "api/value", title: "Value", description: "The dynamic value type stored in tables: variants, conversions, comparison and formatting.", section: "API Reference" },
      { slug: "api/key", title: "Key, KeyRef and AsKey", description: "The owned key type, its borrowed form, and the trait every keyed method accepts.", section: "API Reference" },
      { slug: "api/error", title: "Error", description: "The error enum and Result alias returned by fallible table operations.", section: "API Reference" },
      { slug: "api/table-macro", title: "table!", description: "Build tables with literal syntax.", section: "API Reference" },
    ],
  },
];

export const PAGES: DocPage[] = NAV.flatMap((g) => g.pages);

export function pageHref(slug: string): string {
  return slug === "" ? "/" : `/${slug}/`;
}

export function findPage(slug: string): DocPage | undefined {
  return PAGES.find((p) => p.slug === slug);
}
