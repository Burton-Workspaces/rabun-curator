# Wiki schema

This example vault follows the Karpathy LLM Wiki pattern. `raw/` is immutable. `wiki/` is LLM-owned. Read this file before ingest, query, or lint.

- Never modify `raw/`. `rabun-warehouse` may populate it.
- Update `index.md` on every ingest.
- Append to `log.md` with `## [YYYY-MM-DD] kind | Title`.
- Cite claims via `wiki/sources/` pages.
- Use `[[wikilinks]]`.
