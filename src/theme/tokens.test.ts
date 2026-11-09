import { readFileSync } from "node:fs";

test("tokens.css is identical to the design handoff", () => {
  const app = readFileSync("src/styles/tokens.css", "utf8");
  const handoff = readFileSync("design/tokens.css", "utf8");
  expect(app).toBe(handoff);
});
