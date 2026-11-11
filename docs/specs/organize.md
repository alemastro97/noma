# Organize

**Status:** In progress (unreleased) · **Phase:** 4

## Scope

Keeping things in order after capture. Model-backed features use the same on-device model as capture (`AiService::complete_json`, JSON-schema constrained, temperature 0); the database lock is never held during a model call. Code: `src-tauri/src/organize/*`, UI in `src/views/organize/*`.

1. **Re-sort Inbox.** A _Re-sort Inbox_ button above the Inbox view re-runs filing for open Inbox cards that came from a finished capture (oldest first, at most 25 per run). Each card's original text goes through the capture prompt and schema with today's projects, personal boards, their recent cards and the user's filing corrections (`filing_examples`). Only the place changes (`db::filing::move_item`): project first, then board; no place → it stays. A card the user moved meanwhile is left alone. A toast says how many moved. Re-sorting does not record new filing examples.
2. **Duplicate detection.** After a capture is applied, each new card is compared with open cards (not done, not project cards, not other cards of the same capture) using the stems from `ai/related.rs`: identical words, or ≥ 2 shared stems covering ≥ 60 % of both titles. A match adds `similarTo` / `similarTitle` to the card's hint: the card shows a _Looks like …_ chip, and the detail panel offers **Merge** (the duplicate's title and notes are appended to the existing card's notes, tags united, missing due/remind dates taken over, links moved; then the duplicate is deleted) or **Keep both** (clears the hint). No model call.
3. **Break down.** _Break down_ in the detail panel of an open task asks the model for 3–7 next steps (short imperative headlines, the task's language). They are created as task cards in the same project (or the same personal board; else the Inbox), with the parent's status (Inbox when the parent is done), each linked to the parent with label `step`. Numbering and duplicates are stripped; a copy of the parent title is dropped.
4. **Project summary.** _Summarize_ on a project writes a 2–4 sentence status paragraph (done recently, open, next) from up to 12 cards completed in the last 30 days and 20 open cards. It is stored and shown at the top of the project view; _Regenerate summary_ rewrites it.
5. **Next action.** Per project, the model picks the single next action from the open tasks, reminders and events (numbered list; the schema limits `pick` to a valid index) and phrases it in a few words; shown as **Next** at the top of the project view, click opens the card. One open card needs no model call. When that card is finished or deleted the suggestion disappears (stale) until asked again.
6. **Weekly review.** Sidebar entry **Review**: completed in the last 7 days, overdue (open tasks/reminders whose due or remind time has passed; events are just over), forgotten (open cards not updated for 30+ days, overdue ones excluded). _Write my summary_ asks the model for exactly three warm sentences about the week. Rows open the card. Lists refresh on `items:changed`.
7. **Tag reuse.** When the user has tags, capture offers the 40 most-used in the prompt and lets each card carry up to 2 of them (`tags` constrained to an enum of existing names). In Rust each model tag snaps to the existing spelling (case, `#`, punctuation, or one edit away for 5+ letters) and anything else is dropped. With no tags the prompt and schema are unchanged.
8. **Bigger model.** `src-tauri/models.json` adds **Qwen3 8B Q4_K_M** (`unsloth/Qwen3-8B-GGUF`, 5.03 GB, sha256 `120307ba…49bd4`, verified against the Hugging Face LFS metadata), thinking disabled per request, ~6.5 GB RAM; Settings → On-device AI shows size and RAM on every model card. Not recommended by default and not yet evaluated (ADR-0003 eval should be run before recommending it).

## Data

Migration `0006_organize.sql`: `project_briefs (project_id PK → projects ON DELETE CASCADE, summary, summary_at, next_action, next_item_id → items ON DELETE SET NULL, next_at)`. `CaptureHint` gains optional `similarTo` / `similarTitle` (stored in the existing `items.hint` JSON, no migration).

Commands: `organize_resort_inbox` → `{checked, moved}`, `organize_merge(duplicate, into)` → Item, `organize_break_down(itemId)` → new ids, `organize_brief_get(projectId)`, `organize_summarize_project(projectId)`, `organize_next_action(projectId)` → ProjectBrief, `organize_review` → `{completed, overdue, forgotten}`, `organize_review_summary` → string. Browser dev mode: `src/dev/mock/organize.ts`.

## Decisions

- Summaries, next actions and the review summary run only when asked: no background model calls, so nothing competes with captures or wakes the model unexpectedly.
- Duplicates are lexical and only hinted, never merged automatically: a wrong merge loses words; a wrong hint costs one click.
- Tags: the model never invents tags (a small model scatters near-synonyms); new tags stay a user decision.
- Re-sort reuses the capture prompt unchanged so its eval still describes its behaviour.
- The 8B model is opt-in and unevaluated; the 4B stays the recommended default.

## Changelog

- 2026-11-11 — created: re-sort Inbox, duplicate detection, break down, project summary and next action, weekly review, tag reuse, Qwen3 8B option.
