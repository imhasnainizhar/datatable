import createMDX from "@next/mdx";

const withMDX = createMDX({
  options: { remarkPlugins: [["remark-gfm", {}]] },
});

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Static HTML in out/, ready for GitHub Pages or any static host.
  output: "export",
  trailingSlash: true,
  // Set DOCS_BASE_PATH=/flextable when serving from a GitHub Pages project URL.
  basePath: process.env.DOCS_BASE_PATH || "",
  images: { unoptimized: true },
  pageExtensions: ["ts", "tsx", "mdx"],
};

export default withMDX(nextConfig);
