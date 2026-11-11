/** True when the user asked macOS to reduce motion (Accessibility → Display). */
export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  );
}

/** An animation duration in ms, or 0 when motion should be reduced. */
export function motionMs(ms: number): number {
  return prefersReducedMotion() ? 0 : ms;
}
