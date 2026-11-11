# NOMA

[![CI](https://github.com/alemastro97/noma/actions/workflows/ci.yml/badge.svg)](https://github.com/alemastro97/noma/actions/workflows/ci.yml)

**YOUR SECOND MIND.**

NOMA is a personal AI desktop companion. A small violet orb lives on your desktop; press a shortcut, say or type something, and a correctly typed card — task, reminder, note, idea, project, event — appears on the right board with the right date and reminder. Speech-to-text and the language model run on your machine; your data stays in a local SQLite database.

Stack: Tauri 2 (Rust) · React 19 + TypeScript + Vite · SQLite (`rusqlite`) · llama.cpp sidecar · whisper.cpp.

## Quick start

```bash
pnpm install
pnpm tauri dev
```

Requirements: Node 22, pnpm 10, Rust stable, Xcode Command Line Tools (macOS).

## Development

```bash
pnpm test        # frontend unit tests (Vitest)
pnpm lint        # ESLint
pnpm typecheck   # tsc
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```

## Documentation

- [Spec index](docs/specs/README.md) — living specs, one per feature, with status.
- [Design document](docs/specs/2026-11-09-noma-design.md) — intent, decisions, phasing.
- [Architecture decisions](docs/adr/README.md).
- [Design handoff](design/README.md) — tokens, NomaOrb CSS, screen mockups. Normative for the UI.

## Releases

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, add a CHANGELOG entry, then `git tag vX.Y.Z && git push --tags`. Tagging `v*` builds a universal macOS DMG via GitHub Actions. v1 builds are **unsigned**: on first launch macOS will block the app. Either right-click → Open, or run:

```bash
xattr -dr com.apple.quarantine /Applications/NOMA.app
```

Windows and Linux builds are produced by the same workflow but are not yet supported.

## License

[PolyForm Noncommercial 1.0.0](LICENSE): the source is open to read, run, learn from and modify for personal and other noncommercial use. Selling it, or using it in a commercial product or service, needs the author's permission.
