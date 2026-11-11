/**
 * Mock backend for the "Your data" commands (restore, import, mirror, CLI,
 * MCP, capture history, stats). Browser dev mode only.
 */
import type { CaptureRow, Snapshot, Stats } from "../../api/data";

export const NOT_HANDLED = Symbol("not handled");

const DAY = 86_400_000;
const ago = (ms: number) => new Date(Date.now() - ms).toISOString();

const snapshots: Snapshot[] = Array.from({ length: 4 }, (_, i) => ({
  name: `noma-2026-11-${String(11 - i).padStart(2, "0")}-090000.db`,
  at: ago(i * DAY + 3_600_000),
  sizeBytes: 420_000 - i * 9_000,
}));

const mirror: { enabled: boolean; folder: string | null } = { enabled: false, folder: null };

const captures: CaptureRow[] = [
  {
    id: "c1",
    source: "voice",
    rawText: "Book the photographer for the launch shoot next Monday",
    status: "done",
    latencyMs: 2140,
    createdAt: ago(20 * 60_000),
    itemId: "m1",
    itemTitle: "Book the photographer",
  },
  {
    id: "c2",
    source: "text",
    rawText: "idea: a weekly digest of finished tasks",
    status: "failed",
    latencyMs: 30_000,
    createdAt: ago(3 * 3_600_000),
    itemId: null,
    itemTitle: null,
  },
  {
    id: "c3",
    source: "text",
    rawText: "call the bank about the card",
    status: "done",
    latencyMs: 1480,
    createdAt: ago(DAY),
    itemId: null,
    itemTitle: null,
  },
];

function stats(): Stats {
  const monday = new Date();
  monday.setHours(12, 0, 0, 0);
  monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
  const weeks = Array.from({ length: 8 }, (_, i) => {
    const d = new Date(monday.getTime() - (7 - i) * 7 * DAY);
    return {
      start: d.toISOString().slice(0, 10),
      captures: [6, 9, 4, 12, 8, 15, 11, 7][i] ?? 0,
      completed: [3, 7, 5, 9, 6, 10, 12, 4][i] ?? 0,
    };
  });
  return { weeks, avgLatencyMs: 1830 };
}

const count = (content: unknown) =>
  String(content ?? "")
    .split(/\n\s*\n|\n(?=\s*[-*+] )/)
    .filter((s) => s.trim()).length;

export function handleData(cmd: string, a: Record<string, unknown>): unknown {
  switch (cmd) {
    case "backup_list":
      return snapshots;
    case "backup_restore":
      return null;
    case "import_text": {
      const n = count(a.content);
      return { created: n, skipped: 0, ids: [] };
    }
    case "import_apple_sources":
      return a.source === "notes" ? ["Notes", "Recipes", "Work"] : ["Reminders", "Groceries"];
    case "import_apple":
      return { created: 5, skipped: 0, ids: [] };
    case "pick_folder":
      return "/Users/me/Obsidian/Vault";
    case "mirror_get":
      return { ...mirror, written: null };
    case "mirror_set": {
      if (typeof a.enabled === "boolean") mirror.enabled = a.enabled;
      if (typeof a.folder === "string") mirror.folder = a.folder;
      const written = mirror.enabled && mirror.folder ? `${mirror.folder}/noma.md` : null;
      return { ...mirror, written };
    }
    case "cli_info":
    case "cli_install":
      return {
        script:
          '#!/bin/sh\n# noma — capture into NOMA from the terminal.\nopen -g "noma://capture?text=…"\n',
        path: "/Users/me/.local/bin/noma",
        installed: cmd === "cli_install",
      };
    case "mcp_info":
      return {
        command: "/Applications/NOMA.app/Contents/MacOS/noma-mcp",
        exists: true,
        dbPath: "/Users/me/Library/Application Support/com.noma.app/noma.db",
      };
    case "captures_recent":
      return captures.slice(0, Number(a.limit ?? 50));
    case "captures_retry": {
      const c = captures.find((c) => c.id === a.id);
      if (c) c.status = "pending";
      return null;
    }
    case "stats_get":
      return stats();
    default:
      return NOT_HANDLED;
  }
}
