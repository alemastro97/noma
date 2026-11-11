# Semantic search & Ask NOMA

**Status:** In progress (built, not yet verified in the app) · **Phase:** 4

## Scope

1. **Embedding model.** **EmbeddingGemma 300M Q8_0** (`ggml-org/embeddinggemma-300M-GGUF`, 334 MB, multilingual incl. Italian/English, 768 dims, 2048-token window), pinned by URL + SHA-256 in `src-tauri/models.json` under `embedding` (checksum verified by downloading the file). Downloaded on demand from Settings → Search by meaning, resumable and checksum-verified like the other models; the llama.cpp server build is downloaded too when it is missing (e.g. Ollama users).
2. **Embedding server.** A second `llama-server` (same binary as chat) started lazily with `--embedding`, ctx/batch/ubatch 2048 (a whole card fits one micro-batch), Metal, 1 slot, `--offline`; its own log/pid `ai/logs/llama-embed.log|pid` (a stale one is reaped at start-up). Stopped after 5 minutes without use and when NOMA quits.
3. **Index.** Every card's `title + "\n" + body` (≤ 2000 chars, body skipped when equal to the title) is embedded with the model's document prompt (`title: none | text: `), L2-normalised, and stored as a little-endian f32 blob in `embeddings` with a SHA-256 of model id + text. Item changes (`items:changed`) nudge a background worker, which waits 2 s, then embeds only the cards whose hash changed (batches of 16) and prunes rows of deleted cards / other models. Brute-force cosine over an in-memory copy of the index (dropped after each write).
4. **Palette.** Typing ≥ 3 characters also runs `semantic_search` (250 ms debounce, query prompt `task: search result | query: `, cosine ≥ 0.42, 6 results). Semantic matches the full-text search did not find are appended to the Cards section (max 10 cards) with the hint _Related_. Without the model the call returns nothing and the palette behaves as before.
5. **Ask NOMA.** A chat-style panel opened from the header **Ask** button, the palette command _Ask NOMA_, or _Ask NOMA: "…"_ when the palette query ends with `?` (other unmatched queries still create a task on Enter). Each question retrieves up to 8 cards: by meaning (cosine ≥ 0.3) then topped up with a full-text any-word match (words ≥ 4 letters), so it also works before the embedding model is installed. The local chat model (`AiService::complete_json`, same provider as capture) answers with JSON `{answer, sources}`: 1–4 sentences in the question's language, citing cards as `[n]`. Citations become clickable chips and a _From your cards_ list; clicking opens the card in its place. Single question/answer turns; the last 3 answered turns are sent as context, 8 kept on screen; _New conversation_ clears them. No related card → "I couldn't find anything…" without calling the model.
6. **Settings → Search by meaning.** Model card with size/RAM, download progress / cancel, _Rebuild index_, _Remove_ (deletes model + index), a switch (setting `semantic.enabled`, default on) and a status line (_Ready · n of m cards indexed_ / _Indexing…_ / errors).

## Data

Migration `0007_semantic`: `embeddings(item_id PK → items ON DELETE CASCADE, model_id, content_hash, dims, vector BLOB, updated_at)`. Settings key: `semantic.enabled` (`"1"`/`"0"`). Event: `semantic:state` (snapshot: enabled, ready, model, serverInstalled, download, indexing, indexed, total, running, error). Commands: `semantic_state`, `semantic_configure`, `semantic_download`, `semantic_cancel_download`, `semantic_remove`, `semantic_reindex`, `semantic_search`, `ask_noma`.

## Decisions

- **EmbeddingGemma over Qwen3-Embedding-0.6B**: half the size (334 vs 639 MB) with good multilingual retrieval; both run on the pinned llama.cpp build.
- **Separate server process** rather than `--embedding` on the chat server: the chat server keeps its memory-saving flags and its idle timer, and semantic search works with Ollama as the chat provider.
- **Brute force** cosine: a personal workspace has thousands of cards at most (768 floats each ≈ 3 MB per 1000 cards); no vector index needed.
- **Hash-based re-indexing** instead of tracking ids: a missed event heals on the next pass, and a model change re-embeds everything automatically.
- **Ask NOMA uses schema-constrained JSON** like capture, so small models can't drift out of format; citations are also parsed from `[n]` markers in the text.
- Similarity thresholds (0.42 palette, 0.3 Ask) are first guesses; tune them against real cards.

## Open questions

- Thresholds and answer quality need an in-app check with real cards (not yet run).
- Streaming answers and multi-turn chat are out of scope for v1.

## Changelog

- 2026-11-11 — created: embedding model, background index, semantic palette results, Ask NOMA panel, settings.
