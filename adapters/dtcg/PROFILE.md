# Retentive DTCG flat scalar-token profile zero

Status: executable integrated profile (`nuif-dtcg-scalar-0`). The library,
public CLI, blocking `xtask` gate and CI artifact path exercise the same
profile boundary.

## Model projection

The profile is a bounded subset of the Design Tokens Format Module 2025.10:

- the root is a flat JSON object with one required `$extensions` member and
  zero or more token members;
- each token has exactly `$type`, `$value` and `$extensions` members;
- DTCG `boolean`, `string` and `number` values map to NUIF Boolean, String,
  Integer and finite Real property values;
- token names obey the DTCG name grammar and become NUIF token names;
- root `org.nuif` metadata carries the profile and NUIF document identity;
- token `org.nuif` metadata carries stable token identity and a `value_kind`
  discriminator so DTCG `number` does not collapse NUIF Integer and Real;
- unknown third-party members inside root and token `$extensions` objects are
  retained byte-for-byte by synchronization.

The profile excludes groups, inherited type, `$root`, `$extends`, aliases,
JSON Pointer references, descriptions, deprecation, composite values and
token-local standard members beyond the declared three. These constructs are
not silently treated as scalar tokens. A full DTCG profile requires a token
model RFC for declared type, groups, aliases, descriptions, deprecation and
token-local opaque extensions.

## Retentive laws

For a document inside the profile:

1. `import_source(export_document(document).source).document == document`.
2. Document identity and every token identity, name, declared type, value and
   NUIF value-kind discriminator have UTF-8 byte spans.
3. Synchronization validates the retained source and all correspondence spans
   before applying any edit.
4. Changed spans are replaced from the highest byte offset to the lowest.
5. The synchronized source re-imports to the requested document exactly.
6. Repeated synchronization from the same retained source and edited document
   produces identical source, edit records and reports.
7. Document identity and token inventory changes return typed atomic errors.

Mapped strings, object keys and numbers use one canonical JSON spelling. This
restriction makes stale-span comparison unambiguous while whitespace and
unknown extension values remain retentive.

## Resource and parser contract

Input is UTF-8 and limited to 1 MiB, 4,096 tokens and the `serde_json` default
recursion limit of 128. Duplicate root and token members fail parsing. Token
identities must be unique. Non-finite values are not valid JSON and do not
enter the model. The crate tests cover all four mapped NUIF scalar variants,
exact export/import, deterministic multi-property synchronization, root and
token extension retention, complete unchanged-byte locality, structural and
unsupported edits, stale source, aliases, duplicate members, excessive depth
and the source byte limit.

The command surface is:

- `nuif export <input.nuif> dtcg-scalar-0 <output.tokens.json> [report.json]`;
- `nuif import dtcg-scalar-0 <input.tokens.json> <output.nuif> [report.json]`;
- `nuif sync dtcg-scalar-0 <retained.tokens.json> <edited.nuif> <output.tokens.json> [report.json]`.

`nuif-dtcg-scalar-0` is accepted as an explicit alias. `cargo xtask
gate-dtcg` adds an eight-edit model trial, one-over token-count case and
public-CLI export/sync/import bridge. It writes `target/dtcg-sync-report.json`,
`target/dtcg-sync-output.tokens.json`, `target/dtcg-sync-edited.nuif`,
`target/dtcg-sync-cli-report.json` and
`target/dtcg-sync-cli-output.tokens.json`.
