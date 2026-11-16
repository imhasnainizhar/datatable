import type { MDXComponents } from "mdx/types";
import { mdxComponents } from "@/components/Mdx";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return { ...components, ...(mdxComponents as MDXComponents) };
}
