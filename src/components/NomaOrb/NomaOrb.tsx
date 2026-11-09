import "./noma-orb.css";

export const ORB_STATES = ["idle", "listening", "processing", "success", "error"] as const;
export type OrbState = (typeof ORB_STATES)[number];

export interface NomaOrbProps {
  state?: OrbState;
  /** Diameter in pixels. The handoff CSS scales everything from font-size. */
  size?: number;
  label?: string;
  className?: string;
}

export function NomaOrb({ state = "idle", size = 56, label, className }: NomaOrbProps) {
  const classes = ["noma-orb", `s-${state}`, className].filter(Boolean).join(" ");
  return (
    <div
      className={classes}
      style={{ fontSize: `${size}px` }}
      role="img"
      aria-label={label ?? `NOMA is ${state}`}
    >
      <span className="ring" />
      <span className="ring r2" />
      <span className="ring r3" />
      <span className="orbit">
        <span />
      </span>
      <span className="orbit o2">
        <span />
      </span>
      <span className="core" />
      <span className="err" />
    </div>
  );
}
