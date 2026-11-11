# NOMA specs

Living specifications. One file per feature; each is updated in the same PR as the code it describes. Status values: **Draft** → **In progress** → **Shipped** (or **Superseded**).

| Spec                                         | Status   | Phase |
| -------------------------------------------- | -------- | ----- |
| [Design document](2026-11-09-noma-design.md) | Approved | —     |
| [Foundation](foundation.md)                  | Shipped  | 0     |
| [Workspace & views](workspace-views.md)      | Shipped  | 1     |
| [AI capture](ai-capture.md)                  | Shipped  | 2     |
| [Desktop presence](desktop-presence.md)      | Shipped  | 3     |
| [Time](time.md)                              | Shipped  | 4     |

Plans live in [`../plans/`](../plans/), decisions in [`../adr/`](../adr/).

## Spec template

```markdown
# <Feature>

**Status:** Draft | In progress | Shipped · **Phase:** N · **Plan:** ../plans/<file>.md

## Scope

## UX

## Data

## Open questions

## Changelog

- YYYY-MM-DD — created
```
