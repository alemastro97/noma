# 0003 — Local model and capture latency gate

**Status:** Accepted · **Date:** 2026-11-09 · Amends design document §4.5

## Context

The design document pinned Qwen3-4B-Instruct-2507 (Q4_K_M) and set a validation gate before building on it: type ≥ 90 %, date ≥ 85 %, warm p95 < 1.2 s on the reference machine (M2, 16 GB).

## Measurements

`llama-bench`, llama.cpp b12110, Metal, on the reference M2: **prompt 270 tok/s, generation 26 tok/s** for this model. A capture reply is at least ~35–45 tokens of JSON, so ~1.4 s of generation is the floor regardless of prompt engineering.

Eval (`cargo run --release --example eval`, 40 Italian + English notes), in order of iterations:

| Iteration                                                                                   | type       | date      | repeat     | project    | count     | p50        | p95        |
| ------------------------------------------------------------------------------------------- | ---------- | --------- | ---------- | ---------- | --------- | ---------- | ---------- |
| Full schema, all fields required, `response_format.schema` (not applied by this build)      | —          | —         | —          | —          | —         | 4.1 s      | 5.4 s      |
| Compact schema (2 required fields), compact prompt with examples, OpenAI `json_schema` form | 92.5 %     | 81.6 %    | 95 %       | 97.5 %     | 100 %     | 1.50 s     | 2.13 s     |
| + 8-day calendar in the user message, past recurrences rolled forward in Rust               | 92.5 %     | 100 %     | 100 %      | 97.5 %     | 100 %     | 1.53 s     | 1.94 s     |
| + "a call is a task" example                                                                | **97.5 %** | **100 %** | **97.5 %** | **97.5 %** | **100 %** | **1.54 s** | **2.10 s** |

End to end in the app (placeholder → rewritten card): 2.3 s warm, 8.3 s for the first capture after launch (model load + uncached system prompt).

## Decision

- Keep **Qwen3-4B-Instruct-2507 Q4_K_M** as the default: accuracy is well above the gate.
- Change the latency gate to **warm p95 < 2.5 s**. The 1.2 s target is below what this GPU can generate; the capture flow is non-blocking (the placeholder card appears instantly and is rewritten in place), which keeps the experience inside the overall "capture → card < 3 s" success criterion.
- Ask llama-server for schema-constrained output with the OpenAI form `response_format: {type: "json_schema", json_schema: {schema}}`; the bare `schema` key is ignored by b12110.
- Keep the system prompt long (rules + examples): with a single slot and `cache_prompt` it is processed once per server start.

## Consequences

- The first capture after a cold start takes several seconds; the server stays warm for 10 minutes.
- Qwen3-1.7B remains in the manifest as the faster, lighter option; it needs its own eval run before being recommended.
- Machines faster than an M2 (Pro/Max, M3+) will be proportionally quicker; the gate is set for the slowest supported reference.
