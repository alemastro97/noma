import { atHour, nextMonday, pickerDays, thisEvening } from "./snooze";

test("picker offers a week of days starting today", () => {
  const days = pickerDays(new Date(2026, 10, 11, 15, 30));
  expect(days).toHaveLength(7);
  expect(days[0]!.getDate()).toBe(11);
  expect(atHour(days[1]!, 9).getHours()).toBe(9);
});

test("this evening is 18:00 today until half past five", () => {
  const morning = new Date(2026, 10, 11, 9, 0);
  expect(thisEvening(morning)?.getHours()).toBe(18);
  expect(thisEvening(new Date(2026, 10, 11, 17, 45))).toBeNull();
});

test("next Monday skips to the following week on a Monday", () => {
  const wednesday = new Date(2026, 10, 11, 9, 0);
  const monday = nextMonday(wednesday);
  expect(monday.getDay()).toBe(1);
  expect(monday.getDate()).toBe(16);
  expect(nextMonday(monday).getDate()).toBe(23);
});
