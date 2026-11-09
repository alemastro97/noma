import { describeRule, presetFor, ruleFor } from "./recurrence";

const monday9 = new Date(2026, 10, 9, 9);

test("ruleFor anchors weekly and monthly presets on the card's date", () => {
  expect(ruleFor("weekly", monday9)).toBe("FREQ=WEEKLY;BYDAY=MO");
  expect(ruleFor("monthly", monday9)).toBe("FREQ=MONTHLY;BYMONTHDAY=9");
  expect(ruleFor("weekdays", monday9)).toBe("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR");
  expect(ruleFor("none", monday9)).toBeNull();
});

test("presetFor round-trips presets", () => {
  for (const p of ["daily", "weekdays", "weekly", "monthly", "yearly"] as const) {
    expect(presetFor(ruleFor(p, monday9))).toBe(p);
  }
  expect(presetFor(null)).toBe("none");
});

test("describeRule reads naturally", () => {
  expect(describeRule("FREQ=WEEKLY;BYDAY=MO")).toBe("every Monday");
  expect(describeRule("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR")).toBe("every weekday");
  expect(describeRule("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO")).toBe("every 2 weeks on Monday");
  expect(describeRule("FREQ=MONTHLY;BYMONTHDAY=9")).toBe("every month on the 9th");
  expect(describeRule("FREQ=MONTHLY;BYMONTHDAY=22")).toBe("every month on the 22nd");
  expect(describeRule("FREQ=DAILY;COUNT=3")).toBe("every day, 3 times");
  expect(describeRule(null)).toBe("");
});
