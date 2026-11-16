import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import type { ReactNode } from "react";
import { TopBar } from "@/components/TopBar";
import { Search } from "@/components/Search";
import { searchIndex } from "@/lib/content";
import "./globals.css";

export const metadata: Metadata = {
  title: { default: "datatable: a flexible table type for Rust", template: "%s | datatable" },
  description: "One dynamic container for Rust that works as an array, a dictionary, a record or a set.",
};

// Geist, the typeface family Next.js uses. next/font downloads it at build time
// and serves it from the site itself, so there are no requests to Google.
const geistSans = Geist({ subsets: ["latin"], variable: "--font-geist-sans", display: "swap" });
const geistMono = Geist_Mono({ subsets: ["latin"], variable: "--font-geist-mono", display: "swap" });

// Applied before paint so a saved theme never flashes the other one.
const themeScript = `try{var t=localStorage.getItem("dt-theme");if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}`;

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable}`} suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body>
        <a className="skip-link" href="#content">
          Skip to content
        </a>
        <TopBar />
        {children}
        <Search index={searchIndex()} />
      </body>
    </html>
  );
}
