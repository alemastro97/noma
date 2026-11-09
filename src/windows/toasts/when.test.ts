import type { Item } from "../../api/types";
import { whenLabel } from "./when";

const now = new Date(2026, 10, 9, 15, 45);

test("reminder toast says how far away it is", () => {
  const item = { remindAt: new Date(2026, 10, 9, 16).toISOString(), snoozedUntil: null } as Item;
  expect(whenLabel(item, now)).toBe("In 15 minutes · 16:00");
});

test("snooze wins over the original time", () => {
  const item = {
    remindAt: new Date(2026, 10, 9, 12).toISOString(),
    snoozedUntil: new Date(2026, 10, 9, 15, 45).toISOString(),
  } as Item;
  expect(whenLabel(item, now)).toBe("Now · 15:45");
});
