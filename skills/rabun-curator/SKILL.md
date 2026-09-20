---
name: rabun-curator
description: Maintain a Karpathy LLM wiki with the rabun-curator MCP server. Ingest immutable raw sources into an LLM-owned wiki, query with citations, lint for drift, and explore the knowledge graph. Use when the user mentions rabun-curator, curator, LLM wiki, ingest, vault lint, or wiki query.
argument-hint: "<command> [args]"
---

# /rabun-curator

Orchestrate Karpathy LLM Wiki workflows with rabun-curator MCP tools.

## Prerequisites

The rabun-curator MCP server must be configured. Verify with `stats`. If it fails, tell the user to run `rabun-curator --setup /path/to/vault` and restart the client.

**Vault layers**

- `raw/` — immutable sources. Never write here.
- `wiki/` — LLM-owned pages (entities, concepts, source summaries, comparisons).
- `index.md` — catalog. Update on every ingest.
- `log.md` — append-only. Entry prefix: `## [YYYY-MM-DD] kind | Title`
- `AGENTS.md` — schema. Follow it.

## Commands

```
/rabun-curator ingest [path]     Process a raw/ source into the wiki
/rabun-curator query <question>  Answer from the wiki; offer to file the answer
/rabun-curator lint              Structural + semantic health check
/rabun-curator search <query>    Deep search with synthesis
/rabun-curator connect [path]    Find and apply missing wikilinks
/rabun-curator graph [note]      Neighborhood or vault-wide structure
/rabun-curator analyze           Communities, visualization, graph report
/rabun-curator status            Vault health overview
```

If no command is given, show this usage summary.

## Clients

Tool names are unprefixed (`search`, `read`, `write`, …). Grok namespaces them as `rabun-curator__search`. Claude Code, Claude Desktop, and Cursor use `search`. Same workflows either way.

---

## ingest

**Purpose:** Compile one raw source into the persistent wiki.

1. Locate the source:
   - If `[path]` is given, use it (must be under `raw/` or treat it as a new source to copy conceptually — still never overwrite `raw/`).
   - Otherwise list `raw/` and pick the newest file not yet summarized under `wiki/sources/`.
2. `read` the source. Discuss key takeaways with the user before writing, unless they asked to batch.
3. Write `wiki/sources/<Title>.md` via `write` (auto-wikilinks).
4. Create or update related `wiki/entities/` and `wiki/concepts/` pages. A single source may touch many pages. Flag contradictions with older claims.
5. Refresh `index.md` (link + one-line summary per page).
6. Append to `log.md`:
   ```
   ## [YYYY-MM-DD] ingest | Title
   - source: raw/...
   - pages touched: ...
   ```
7. Report paths written, auto-links added, and open questions.

Do not modify `raw/`. If `write` refuses a path, stop and correct the destination.

---

## query

**Purpose:** Answer from the compiled wiki, not by rediscovering raw sources from scratch.

1. `read` `index.md`.
2. `search` the question; `read` the top pages (and `links` / `traverse` when a neighborhood would help).
3. Synthesize an answer with `[[wikilink]]` citations.
4. Offer to file a strong answer as `wiki/concepts/` or `wiki/comparisons/`. If the user agrees, `write` it, update `index.md`, append `## [YYYY-MM-DD] query | Title` to `log.md`.

---

## lint

**Purpose:** Keep the wiki healthy as it grows.

1. Call `lint` and `stats`.
2. Semantic pass (you, not the tool): contradictions between pages, stale claims superseded by newer sources, concepts mentioned but lacking a page, missing cross-references.
3. Propose fixes. Apply only with user confirmation, except trivial missing `[[wikilinks]]` the user already asked to connect.
4. Append `## [YYYY-MM-DD] lint | summary` to `log.md` when a pass completes.

---

## search

Call `search` with the query. `read` the top few hits. Summarize what the wiki already knows and where the gaps are. Do not dump raw snippets without synthesis.

---

## connect

1. If `[path]` is given, `suggest_links` on that file; otherwise `stats` then pick poorly linked wiki pages.
2. Show suggestions. Apply with `write` only after confirmation unless the user said to apply them.

---

## graph

- With `[note]`: `traverse` from that stem (depth 2 unless asked otherwise). Optionally `shortest_path` between two named notes.
- Without: `cluster` plus hub notes from `stats` / a short `report`.

---

## analyze

1. `cluster`
2. `visualize` (writes `GRAPH_VIZ.html`)
3. `report` (writes `GRAPH_REPORT.md`)
4. Summarize god nodes, communities, orphans, and 2–3 questions the graph is positioned to answer.

---

## status

Call `stats` and `lint`. Report file/word/link counts, orphan count, broken links, raw sources missing `wiki/sources/` summaries, and pages absent from `index.md`. Keep it short.

---

## Tool reference

| Tool | Use |
|------|-----|
| `search` | Full-text search |
| `read` / `write` / `list` | Files (`write` refuses `raw/`) |
| `links` / `suggest_links` / `metadata` | Wikilinks and frontmatter |
| `stats` / `lint` | Health |
| `traverse` / `shortest_path` | Graph walk |
| `cluster` / `visualize` / `report` | Communities and viz |
