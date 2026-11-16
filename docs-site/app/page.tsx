import type { Metadata } from "next";
import { DocPage } from "@/components/DocPage";
import { findPage } from "@/lib/nav";

export const metadata: Metadata = {
  title: { absolute: "datatable: a flexible table type for Rust" },
  description: findPage("")!.description,
};

export default function Home() {
  return <DocPage slug="" />;
}
