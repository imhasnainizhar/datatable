import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { DocPage } from "@/components/DocPage";
import { PAGES, findPage } from "@/lib/nav";

type Props = { params: Promise<{ slug: string[] }> };

export const dynamicParams = false;

export function generateStaticParams() {
  return PAGES.map((p) => ({ slug: p.slug.split("/") }));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const page = findPage((await params).slug.join("/"));
  return page ? { title: page.title, description: page.description } : {};
}

export default async function Page({ params }: Props) {
  const slug = (await params).slug.join("/");
  if (!findPage(slug)) notFound();
  return <DocPage slug={slug} />;
}
