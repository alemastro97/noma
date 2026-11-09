export interface Command {
  id: string;
  label: string;
  section: "goTo" | "actions";
  keywords?: string;
  hint?: string;
  run: () => void;
}

function score(cmd: Command, q: string): number {
  const label = cmd.label.toLowerCase();
  const hay = `${label} ${cmd.keywords?.toLowerCase() ?? ""}`;
  if (label.startsWith(q)) return 3;
  if (label.split(/\s+/).some((w) => w.startsWith(q))) return 2;
  if (hay.includes(q)) return 1;
  // Subsequence match ("nwtsk" → "New task").
  let i = 0;
  for (const ch of hay) if (ch === q[i]) i++;
  return i === q.length ? 0.5 : 0;
}

/** Commands matching `query`, best first; all commands in original order when empty. */
export function filterCommands(commands: Command[], query: string): Command[] {
  const q = query.trim().toLowerCase();
  if (!q) return commands;
  return commands
    .map((c, i) => ({ c, s: score(c, q), i }))
    .filter((x) => x.s > 0)
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .map((x) => x.c);
}
