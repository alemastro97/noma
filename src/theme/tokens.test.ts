import { readFileSync } from "node:fs";

type Blocks = Record<string, Record<string, string>>;

/** Parses `selector { --token: value; }` blocks, ignoring comments. */
function parse(css: string): Blocks {
  const out: Blocks = {};
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const m of clean.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
    const tokens: Record<string, string> = {};
    for (const t of m[2]!.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) tokens[t[1]!] = t[2]!.trim();
    out[m[1]!.trim()] = tokens;
  }
  return out;
}

/**
 * Deliberate departures from the handoff, documented in
 * docs/specs/accessibility.md (contrast): small text reaches 4.5:1.
 */
const A11Y_OVERRIDES: Blocks = {
  ":root": { "--noma-text-muted": "#8C8698" },
  '[data-theme="light"]': {
    "--noma-text-2": "#6B6575",
    "--noma-text-muted": "#6E6878",
    "--noma-lavender": "#7255D0",
    "--noma-reminder-text": "#96600F",
    "--noma-urgent-text": "#BD3F47",
  },
};

test("tokens.css matches the design handoff apart from the documented contrast overrides", () => {
  const app = parse(readFileSync("src/styles/tokens.css", "utf8"));
  const handoff = parse(readFileSync("design/tokens.css", "utf8"));
  const expected: Blocks = {};
  for (const [sel, tokens] of Object.entries(handoff)) {
    expected[sel] = { ...tokens, ...A11Y_OVERRIDES[sel] };
  }
  expect(app).toEqual(expected);
});
