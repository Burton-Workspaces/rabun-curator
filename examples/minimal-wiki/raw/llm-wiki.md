# LLM Wiki (source excerpt)

A pattern for personal knowledge bases using LLMs.

Most LLM-and-documents workflows look like RAG: upload files, retrieve chunks at query time, generate an answer. The model rediscovers knowledge from scratch on every question. Nothing accumulates.

The wiki pattern is different. The LLM incrementally builds a persistent wiki of markdown files between you and the raw sources. When a source arrives, it is read, integrated, and cross-referenced. Contradictions are flagged. Synthesis reflects everything already ingested.

Three layers: immutable raw sources, an LLM-owned wiki, and a schema (AGENTS.md) that tells the model how to ingest, query, and lint.

Operations: ingest, query, lint. Navigation: index.md (catalog) and log.md (timeline).
