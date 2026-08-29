# 12 — CLI, API and automation surface

Status: draft.

A conforming reference implementation MUST expose semantic operations without GUI automation.

Required command classes: `inspect`, `query`, `validate`, `canonicalize`, `diff`, `patch`, `render`, `layout`, `snapshot`, `migrate`, `capabilities`, `replay`, `import` and `export`.

Every command MUST support machine-readable output and stable diagnostic codes. Headless commands SHOULD accept stdin/stdout for pipeline use.

The editor MUST route mutations through the same operation layer available to CLI/API clients. AI/MCP adapters are optional clients of this interface and are never the canonical protocol.
