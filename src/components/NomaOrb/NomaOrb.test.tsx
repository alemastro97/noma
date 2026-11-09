import { readFileSync } from "node:fs";
import { render, screen } from "@testing-library/react";
import { NomaOrb, ORB_STATES } from "./NomaOrb";

test.each(ORB_STATES)("renders state class s-%s", (state) => {
  render(<NomaOrb state={state} />);
  expect(screen.getByRole("img")).toHaveClass("noma-orb", `s-${state}`);
});

test("defaults to idle", () => {
  render(<NomaOrb />);
  expect(screen.getByRole("img")).toHaveClass("s-idle");
});

test("sizes via font-size", () => {
  render(<NomaOrb size={32} />);
  expect(screen.getByRole("img")).toHaveStyle({ fontSize: "32px" });
});

test("default aria-label names the state", () => {
  render(<NomaOrb state="listening" />);
  expect(screen.getByRole("img", { name: "NOMA is listening" })).toBeInTheDocument();
});

test("custom label wins", () => {
  render(<NomaOrb label="AI status: idle" />);
  expect(screen.getByRole("img", { name: "AI status: idle" })).toBeInTheDocument();
});

test("renders the handoff markup (3 rings, 2 orbits, core, err)", () => {
  const { container } = render(<NomaOrb />);
  expect(container.querySelectorAll(".ring")).toHaveLength(3);
  expect(container.querySelectorAll(".orbit")).toHaveLength(2);
  expect(container.querySelector(".core")).not.toBeNull();
  expect(container.querySelector(".err")).not.toBeNull();
});

test("orb CSS is identical to the design handoff and respects prefers-reduced-motion", () => {
  const css = readFileSync("src/components/NomaOrb/noma-orb.css", "utf8");
  expect(css).toBe(readFileSync("design/noma-orb.css", "utf8"));
  expect(css).toContain("@media (prefers-reduced-motion:reduce)");
  expect(css).toContain("animation:none!important");
});
