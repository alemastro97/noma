/** Project colors: the handoff's semantic accents, in the order new projects cycle through. */
export const PROJECT_COLORS = [
  "#79B8F5", // blue
  "#78DCC0", // mint
  "#F2BD73", // amber
  "#DDA1C5", // rose
  "#A78BFA", // lavender
  "#F28C91", // coral
] as const;

export function nextProjectColor(existing: number): string {
  return PROJECT_COLORS[existing % PROJECT_COLORS.length]!;
}
