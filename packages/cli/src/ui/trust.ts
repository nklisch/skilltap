import type { TapTrust, TrustInfo } from "@skilltap/core";
import { ansi } from "./format";

/** Short label for the list/find table column. Fixed width: up to ~15 chars. */
export function formatTrustTier(trust?: TrustInfo | null): string {
  if (!trust) return ansi.dim("○ unverified");
  switch (trust.tier) {
    case "provenance":
      return ansi.green("✓ provenance");
    case "publisher":
      return ansi.dim("● publisher");
    case "curated":
      return ansi.dim("◆ curated");
    case "unverified":
      return ansi.dim("○ unverified");
  }
}

/** Full label for the info command. */
export function formatTrustLabel(trust: TrustInfo): string {
  switch (trust.tier) {
    case "provenance":
      return ansi.green("✓ Provenance verified");
    case "publisher":
      return ansi.dim("● Publisher known");
    case "curated":
      return trust.tap
        ? ansi.dim(`◆ Curated (${trust.tap})`)
        : ansi.dim("◆ Curated");
    case "unverified":
      return ansi.dim("○ Unverified");
  }
}

/** Plain-text label for agent mode output (no ANSI). */
export function agentTrustLabel(trust: TrustInfo | undefined): string {
  if (!trust) return "unverified";
  switch (trust.tier) {
    case "provenance":
      return `provenance verified`;
    case "publisher":
      return trust.publisher
        ? `publisher: ${trust.publisher.name}`
        : "publisher known";
    case "curated":
      return "curated";
    case "unverified":
      return "unverified";
  }
}

/** Format tap-level trust for find command display. */
export function formatTapTrust(tapTrust?: TapTrust | null): string {
  if (tapTrust?.verified) return ansi.dim("◆ verified");
  return ansi.dim("◆ curated");
}
