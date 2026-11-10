/** Bar heights (0.15..1) for the waveform from recent input levels. */
export function bars(levels: number[], count: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < count; i++) {
    const v = levels[levels.length - count + i] ?? 0;
    out.push(Math.max(0.15, Math.min(1, Math.sqrt(v) * 4)));
  }
  return out;
}
