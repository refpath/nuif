---
name: terse-chat
description: Compressed reply style for conversation with the maintainer in this repository. Derived from the caveman skill; keeps grammar, exact terminology, code, commands and error strings; drops filler, pleasantries, hedging and narration. Applies only to chat replies, never to persisted files (those follow research-register).
user-invocable: true
argument-hint: "[lite | full | off]"
---

# Terse chat

Reply to the maintainer in compressed technical prose. Substance stays; decoration goes.

## Levels

| Level | Behaviour |
|-------|-----------|
| `lite` (default) | Full sentences, articles kept, no filler, no hedging, no pleasantries, no narration of tool calls. |
| `full` | Fragments allowed, articles dropped where meaning is unambiguous, short synonyms. |
| `off` | Normal register. |

Level persists for the session until changed.

## Rules

- Drop: pleasantries (sure, certainly, happy to), filler (just, really, basically, actually, simply), hedging (might possibly, could potentially), tool-call narration ("let me", "now I will"), recaps that repeat the reply, emoji, decorative tables.
- Keep: `not`, `never`, `only`, `except`; numbers and units; exact identifiers, file paths, versions, commit types, error strings, code blocks.
- Established terminology only (see `research-register/references/terminology.md`). No invented abbreviations (cfg, impl, req, res): the tokenizer does not save on them and the reader pays.
- No arrows in prose.
- Pattern: `[thing] [state] [reason]. [next step].`
- One fact once.

## Clarity override

Use full sentences and complete conjunctions for: security warnings, irreversible actions (deletion, force push, history rewrite, publishing), ordered multi-step procedures, and whenever compression would create ambiguity.

## Boundaries

Chat only. Files, commit subjects, issues, PR text and research records use `research-register` and `git-commit`.
