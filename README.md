# Curator

Give an LLM a librarian for a **Karpathy LLM Wiki**: immutable raw sources, an LLM-owned markdown wiki, and a schema that keeps ingest / query / lint disciplined.

Curator is a **local MCP server** written in Rust. It searches the vault, auto-`[[wikilinks]]` on write, refuses mutations under `raw/`, walks the knowledge graph, detects communities, and writes a D3-style graph visualization. Claude Code, Grok, and Cursor all get the same `/curator` skill.

Inspired by [Andrej Karpathy's LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) and the Rust architecture of [librarian-mcp](https://github.com/ngmeyer/librarian-mcp) (MIT). Curator is an original implementation, not a fork.

Your vault never leaves the machine. The server speaks MCP over stdio. No network, no telemetry.

## Why a wiki, not RAG

RAG rediscovers knowledge from raw chunks on every question. A wiki **compiles** it: entity pages, concept pages, contradiction flags, and synthesis already reflect everything you have ingested. The human curates sources and asks questions. The LLM does the bookkeeping.

## Quick start

```bash
# Build
cargo install --path .

# Create a vault (raw/, wiki/, index.md, log.md, AGENTS.md)
curator init ~/my-wiki

# Point Claude, Grok, and Cursor at it (skills + MCP)
curator --setup ~/my-wiki
```

Restart the client, then try `/curator status` or `/curator ingest`.

Try the bundled example:

```bash
curator --setup examples/minimal-wiki
```

## CLI

| Command | What it does |
|---------|----------------|
| `curator /path/to/vault` | Start the MCP server on stdio |
| `curator --setup /path/to/vault` | Write MCP config + `/curator` skill for Claude, Grok, and Cursor |
| `curator init /path/to/vault` | Scaffold the three-layer wiki |

Environment: `CURATOR_VAULT` or colon-separated `CURATOR_VAULTS`.

Ignore extra paths with a gitignore-style `.curatorignore`. Extra auto-link stopwords: `.curatorstoplist`.

## MCP tools

| Tool | Description |
|------|-------------|
| `search` | Trigram + BM25 full-text search |
| `read` / `write` / `list` | Files. `write` refuses `raw/` and auto-links titles |
| `links` / `suggest_links` / `metadata` | Wikilinks, suggestions, YAML frontmatter |
| `stats` / `lint` | Health: orphans, broken links, index/log gaps |
| `traverse` / `shortest_path` | Graph neighborhood and shortest chain |
| `cluster` | Louvain communities |
| `visualize` | Interactive HTML graph (`GRAPH_VIZ.html`) |
| `report` | God nodes, communities, bridges (`GRAPH_REPORT.md`) |

Grok names tools `curator__search`, `curator__read`, and so on.

## Skill commands

`--setup` installs [`skills/curator/SKILL.md`](skills/curator/SKILL.md) to:

- `~/.claude/skills/curator/`
- `~/.grok/skills/curator/`
- `~/.cursor/skills/curator/`

| Command | What it does |
|---------|----------------|
| `/curator ingest [path]` | Compile a `raw/` source into wiki pages, index, and log |
| `/curator query <question>` | Answer from the wiki; file good answers back |
| `/curator lint` | Structural lint plus a semantic health pass |
| `/curator search` / `connect` / `graph` / `analyze` / `status` | Search, wikilinks, graph, viz, overview |

## Vault layout

```
vault/
  raw/                 # immutable sources
  wiki/
    entities/
    concepts/
    sources/           # summaries of raw files
    comparisons/
  index.md             # catalog
  log.md               # append-only (`## [YYYY-MM-DD] ingest | Title`)
  AGENTS.md            # schema
```

## License

MIT
