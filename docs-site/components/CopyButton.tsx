"use client";

import { useState } from "react";

export function CopyButton({ text, icon = false }: { text: string; icon?: boolean }) {
  const [copied, setCopied] = useState(false);
  const label = copied ? "Copied" : "Copy";
  return (
    <button
      className={`${icon ? "copy copy-icon" : "copy"}${copied ? " copied" : ""}`}
      type="button"
      aria-label={icon ? `${label} command` : "Copy code"}
      title={icon ? label : undefined}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(text);
        } catch {
          return;
        }
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
    >
      {!icon ? (
        label
      ) : copied ? (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" d="m5 12.5 4.5 4.5L19 7.5" />
        </svg>
      ) : (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <rect x="8.5" y="8.5" width="11" height="11" rx="2" fill="none" stroke="currentColor" strokeWidth="1.8" />
          <path fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" d="M15.5 5.5v-.5a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h.5" />
        </svg>
      )}
    </button>
  );
}
