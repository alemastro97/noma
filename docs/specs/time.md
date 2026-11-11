# Time

**Status:** Shipped (Unreleased) · **Phase:** 4

## Scope

Everything about _when_: seeing cards on a calendar, being reminded at the right moment, doing the work, keeping routines.

1. **Calendar view.** The former "Soon" Calendar button in the view switcher. **Month** grid (whole Monday-first weeks covering the month) and **Week** grid (7 tall columns), with ‹ Today › navigation. Each card sits on the local day of its due date, else its reminder date (project cards are left out), earliest first, as a chip with a type-colored dot, time (when it has one) and title; done cards are struck through. A month cell shows 3 chips, then _+N more_ (opens that week). Click a chip to open its details; **drag** a chip to another day to reschedule — due, end and reminder all shift by the same number of days, keeping their times. **Click an empty part of a day** to quick-add a card due that day at 18:00 (Task preselected; with Auto the AI decides the date).
2. **Timeline view.** The former "Soon" Timeline button. 14 horizontal day columns starting two days before today (‹ Today › moves a week), today tinted. Cards are bars in lanes on their day; **events span from start to `endsAt`** (an end at midnight belongs to the day before); bars running off the range are cut square at the edge. Click a bar to open its details.
   Outside a project or a personal board, Calendar and Timeline show **every** dated card (the list filter `all`); inside a project or board, just its cards.
3. **Custom snooze.** Reminder toasts gain _Later…_ under the existing actions: **This evening** (18:00 today, hidden from 17:30 on), **Next Monday** (09:00; a week ahead on a Monday), and a small mouse-only **picker** — one of the next 7 days × 09:00 / 12:00 / 15:00 / 18:00 / 21:00 (past times disabled), then _Snooze_. The toast window never takes keyboard focus, so there is no text field. Uses `reminder_snooze_until`.
4. **Evening recap.** Once a day, the first time NOMA is awake after 19:00 local, a toast _Your day · N done, M open_ lists up to 3 still-open and 3 done-today cards (Today's list with done items) with _Dismiss_ and _See today_. Skipped on an empty day; paused with reminders; off with Settings → Time → _Evening recap_ (`reminders.recap`, default on). `recap.last_day` remembers the day. Same mechanism as the morning brief.
5. **Focus timer.** _Focus 25 min_ in a card's details, or the timer button beside a row in the orb panel, starts a session (one at a time; starting another logs the running one as stopped). The orb shows a **progress ring**, the orb panel a bar with the title, time left and _Stop_, the details _Focusing · 12:30 left_ and _Stop_. At the end a toast _Focus session done_ offers _25 more_ and _Mark done_ (plus a macOS notification when NOMA is not in front). Sessions are logged per card; the details show _N sessions · M min_ (completed sessions; minutes include stopped ones). The running session lives in memory: quitting NOMA drops it.
6. **Habits.** A repeating card can be marked **Habit** in its details. It then shows a **streak** (_N in a row_) and a **7-day dot strip** (filled = an occurrence was completed that day) in the details and in a **Habits** section at the top of Today (List and Kanban), where it can be ticked off.
7. **Reminders before the due time.** In a card's details, _Remind me before_ toggles 10 min / 1 hour / 1 day / 1 week before its **due** time (needs a due date). Each fires once per due time as a toast _Due Today · 16:00_ with _Done_ and _Later…_ (snoozing sets the card's normal snooze). If several passed at once (the Mac slept) only the closest fires; none fire once the due time has passed.

## Data

Migration `0010_time.sql`:

- `reminder_offsets (item_id, minutes, fired_for)` — PK `(item_id, minutes)`; `fired_for` is the due time it last fired for, so moving the due date re-arms it.
- `habits (item_id PK, created_at)` — only items with a recurrence can be added.
- `focus_sessions (id, item_id, started_at, ended_at, minutes, completed)`.

All cascade on item delete. Streaks come from `item_occurrences` (written when a repeating card is completed).

Commands (`src-tauri/src/timekeeping.rs`, data in `db/timekeeping.rs`): `reminder_snooze_until`, `recap_open`, `time_item_info`, `reminder_offsets_set`, `habit_set`, `habits_list`, `focus_current`, `focus_start`, `focus_stop`. Event `focus:changed` carries the running session (or null) to every window. Toast ids: `pre:<item>`, `focus:<item>`, `recap` (rendered by `src/windows/toasts/TimeToasts.tsx`); the `Toast` struct is unchanged.

A `TimeService` loop beside the reminder scheduler sleeps until the next offset or the end of the focus session (max 30 s, nudged on changes), then fires offsets, the recap and the focus end.

## Decisions

- **Streak = on-time occurrences in a row.** An occurrence counts when it was completed on (or before) its scheduled local day; a late one ends the streak, and a habit still due before today shows 0. This works for any recurrence (daily, weekly…), not only daily ones, because a missed occurrence stays due until completed.
- Calendar and Timeline are lenses like the other views, but a calendar of only "Today" is useless, so they widen to all cards outside projects/boards.
- Reschedule by whole local days (wall-clock time kept across DST).
- New toast kinds use id prefixes instead of new `Toast` fields, so the shared struct and other toast producers stay untouched.
- No hour grid in the week view and no drag in the timeline in v1: cards keep their time, only the day changes.

## Changelog

- 2026-11-11 — created and shipped: calendar and timeline views, custom snooze, evening recap, focus timer, habits, reminders before the due time.
