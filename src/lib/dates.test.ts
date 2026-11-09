import {
  editedAgo,
  eventBlock,
  formatDay,
  formatDue,
  formatReminder,
  fromLocalInput,
  parseIso,
  partOfDay,
  relativeFromNow,
  timeRange,
  toLocalInput,
} from "./dates";

// Monday 9 November 2026, 13:50 local.
const now = new Date(2026, 10, 9, 13, 50);
const at = (d: number, h = 0, m = 0) => new Date(2026, 10, d, h, m);

test("formatDay names nearby days and short dates further out", () => {
  expect(formatDay(at(9, 16), now)).toBe("Today");
  expect(formatDay(at(10), now)).toBe("Tomorrow");
  expect(formatDay(at(8), now)).toBe("Yesterday");
  expect(formatDay(at(12), now)).toBe("Thu 12");
  expect(formatDay(at(30), now)).toBe("Nov 30");
  expect(formatDay(new Date(2027, 3, 2), now)).toBe("Apr 2, 2027");
});

test("formatDue adds the time only when there is one", () => {
  expect(formatDue(at(12, 18), now)).toBe("Thu 12 · 18:00");
  expect(formatDue(at(12), now)).toBe("Thu 12");
});

test("formatReminder leads with the day", () => {
  expect(formatReminder(at(9, 16), now)).toBe("Today, 16:00");
});

test("relativeFromNow reads like the mockup", () => {
  expect(relativeFromNow(at(9, 16), now)).toBe("in 2h 10m");
  expect(relativeFromNow(at(9, 14), now)).toBe("in 10m");
  expect(relativeFromNow(at(9, 11, 50), now)).toBe("2h ago");
  expect(relativeFromNow(at(12, 13, 50), now)).toBe("in 3d");
  expect(relativeFromNow(now, now)).toBe("now");
});

test("editedAgo", () => {
  expect(editedAgo(at(8, 10), now)).toBe("edited yesterday");
  expect(editedAgo(at(9, 13, 45), now)).toBe("edited 5m ago");
});

test("eventBlock and timeRange", () => {
  expect(eventBlock(at(11, 14, 30))).toEqual({ weekday: "WED", day: "11" });
  expect(timeRange(at(11, 14, 30), at(11, 15, 30))).toBe("14:30 – 15:30");
  expect(timeRange(at(11, 14, 30), null)).toBe("14:30");
});

test("partOfDay", () => {
  expect(partOfDay(at(9, 9))).toBe("morning");
  expect(partOfDay(at(9, 14))).toBe("afternoon");
  expect(partOfDay(at(9, 19))).toBe("evening");
});

test("parseIso tolerates nanoseconds and the SQLite space separator", () => {
  expect(parseIso("2026-11-09T10:00:00.123456789Z").getTime()).toBe(
    Date.UTC(2026, 10, 9, 10, 0, 0, 123),
  );
  expect(parseIso("2026-11-09 10:00:00+00:00").getTime()).toBe(Date.UTC(2026, 10, 9, 10));
});

test("local input round-trips", () => {
  const iso = at(12, 18, 5).toISOString();
  expect(toLocalInput(iso)).toBe("2026-11-12T18:05");
  expect(fromLocalInput("2026-11-12T18:05")).toBe(iso);
  expect(fromLocalInput("")).toBeNull();
});
