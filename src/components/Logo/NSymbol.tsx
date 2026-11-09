interface NSymbolProps {
  size?: number;
  className?: string;
}

/** The minimal N symbol from the design handoff. Stroke follows `currentColor`; the dot is lavender. */
export function NSymbol({ size = 22, className }: NSymbolProps) {
  // The handoff thickens the stroke at small sizes: 4.2 (≥96) · 4.6 @64 · 5.4 @32 · 6.5 @16.
  const strokeWidth = size <= 16 ? 6.5 : size <= 32 ? 5.4 : size <= 64 ? 4.6 : 4.2;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      className={className}
    >
      <path
        d="M14 35V15.5a1.5 1.5 0 0 1 2.7-.9L31.3 33.4a1.5 1.5 0 0 0 2.7-.9V20"
        stroke="currentColor"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="34" cy="12.2" r="3.6" fill="var(--noma-lavender)" />
    </svg>
  );
}
