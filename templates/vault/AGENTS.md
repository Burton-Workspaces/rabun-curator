# Wiki schema

This vault is a [Karpathy LLM Wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f): a persistent, compounding knowledge base. You (the LLM) write and maintain the wiki. The human curates sources, directs analysis, and asks questions.

## Layers

- **`raw/`** — immutable sources. Read them. Never modify them.
- **`wiki/`** — LLM-owned markdown: entity pages, concept pages, source summaries, comparisons, syntheses.
- **`AGENTS.md`** — this schema. Co-evolve it with the human when conventions need to change.
- **`index.md`** — catalog of wiki pages with a one-line summary each. Update on every ingest.
- **`log.md`** — append-only operations log.

## Conventions

- Cite factual claims with a link to the ingested source page under `wiki/sources/`.
- Use Obsidian `[[wikilinks]]` (and `[[Note|display]]` when the mention text differs).
- YAML frontmatter on wiki pages: `title`, `tags`, `updated`, `sources` (list of raw/ or wiki/sources/ paths).
- Put entities in `wiki/entities/`, concepts in `wiki/concepts/`, source summaries in `wiki/sources/`, comparisons in `wiki/comparisons/`.
- File good query answers back into the wiki. Chat is ephemeral; the wiki compounds.

## Operations

**Ingest.** Read a new `raw/` file. Discuss takeaways. Write `wiki/sources/<Name>.md`. Update related entity and concept pages. Refresh `index.md`. Append to `log.md`:

```
## [YYYY-MM-DD] ingest | Source Title
```

**Query.** Read `index.md` first, then search and read the relevant pages. Answer with citations. Offer to file the answer as a new wiki page.

**Lint.** Run structural lint, then check for contradictions, stale claims, missing concept pages, and missing cross-references. Propose new questions and sources.

## Tools

Prefer rabun-curator MCP tools (`search`, `read`, `write`, `lint`, `stats`, graph tools) when available. `write` refuses `raw/`. After writes, the search index and graph refresh automatically.
