import { bars } from "./summary";

test("bars are clamped and padded on the left", () => {
  const b = bars([0, 1], 4);
  expect(b).toHaveLength(4);
  expect(b[0]).toBe(0.15);
  expect(b[3]).toBe(1);
});
