# AI capture

**Status:** In progress · **Phase:** 2

## Scope

Free text (quick add in **Auto** mode) becomes typed cards, sorted by a small model running on the Mac.

1. **Engine.** `llama-server` (llama.cpp **b12110** = v0.9.0) downloaded on demand from GitHub Releases, plus a GGUF model from Hugging Face, both pinned by URL + SHA-256 in `src-tauri/models.json`. Downloads resume, verify the checksum, and are ad-hoc signed if needed. Default model: **Qwen3-4B-Instruct-2507 Q4_K_M** (2.5 GB, non-thinking); light option **Qwen3-1.7B Q4_K_M** (thinking disabled per request).
2. **Server lifecycle.** Started lazily on the first capture (Metal, ctx 4096, 1 slot, `--offline`), kept warm 10 minutes, stopped when idle or when NOMA quits. Logs: `ai/logs/llama-server.log`.
3. **Extraction.** `/v1/chat/completions` with `response_format: json_schema` (GBNF-constrained) at temperature 0. The model returns 1–3 items with local wall-clock times and a repeat preset; Rust converts to UTC, validates plausibility (now − 1 day … now + 2 years), builds the RRULE from the anchor's local weekday/day, fuzzy-matches project names (Jaro-Winkler ≥ 0.88 or containment) and records doubts as **hints** on the card: _Check date · phrase_, _New project? name_, _Check this_ (confidence < 0.6).
4. **Capture lifecycle.** A placeholder note appears instantly (shimmer, "Reading…"); the worker rewrites it in place (same id, same canvas spot) and creates any extra cards beside it. Captures made before a model is installed stay pending and run as soon as one is ready; interrupted ones are re-queued at start-up; failures keep the note and offer _Retry_.
5. **Ollama** (developer option): same schema via `/api/chat` `format`, `think: false`.
6. **Settings → On-device AI**: engine choice, model cards with download progress / cancel / use / remove, pending count. Header chip shows Idle · Downloading n% · Waking up · Thinking · Captured.
7. **Eval.** `ai/evals/cases.jsonl` (40 Italian + English notes, fixed now = Mon 9 Nov 2026 10:00 Europe/Rome) and `cargo run --release --example eval`. Gate: type ≥ 90 %, date ≥ 85 %, warm p95 < 1.2 s.

## Data

Migration `0003`: `items.hint` (JSON). Items carry `hint` and `capturing` on the wire. Settings keys: `ai.provider`, `ai.model`, `ai.ollamaModel`, `ai.ollamaUrl`, `ai.keepWarmMinutes`. Events: `ai:state` (full snapshot), `ai:capture` (`done` / `failed`).

## Decisions

- The model outputs a repeat **preset**, not an RRULE: small models are reliable with an enum; Rust builds the rule.
- Local wall-clock times instead of ISO with offsets for the same reason; time zones are Rust's job.

## Changelog

- 2026-11-09 — created
