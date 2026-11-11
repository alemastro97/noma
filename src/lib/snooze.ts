/** Custom snooze targets for reminder toasts. Pure: `now` is passed in. */

/** Hour "this evening" means. */
export const EVENING_HOUR = 18;
/** Hour "next Monday" means. */
export const MORNING_HOUR = 9;

/** Today at 18:00, or null once it is (nearly) evening already. */
export function thisEvening(now: Date): Date | null {
  const d = new Date(now);
  d.setHours(EVENING_HOUR, 0, 0, 0);
  return d.getTime() - now.getTime() >= 30 * 60_000 ? d : null;
}

/** Hours offered by the toast's small picker (toasts take no keyboard, so no text field). */
export const PICKER_HOURS = [9, 12, 15, 18, 21];

/** The next 7 days (today first), at midnight, for the picker. */
export function pickerDays(now: Date): Date[] {
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(now);
    d.setDate(d.getDate() + i);
    d.setHours(0, 0, 0, 0);
    return d;
  });
}

/** `day` at `hour`:00 local. */
export function atHour(day: Date, hour: number): Date {
  const d = new Date(day);
  d.setHours(hour, 0, 0, 0);
  return d;
}

/** The coming Monday at 09:00 (a week ahead when today is Monday). */
export function nextMonday(now: Date): Date {
  const d = new Date(now);
  const ahead = (8 - d.getDay()) % 7 || 7;
  d.setDate(d.getDate() + ahead);
  d.setHours(MORNING_HOUR, 0, 0, 0);
  return d;
}
