# Security policy

NUIF treats documents, assets, extensions and adapter inputs as untrusted.
Binary and text decoders, archive decompression, fonts, images, vector paths,
shader and effect extensions, GPU allocations and collaboration inputs are
security-sensitive surfaces. The normative threat model is
[`spec/11-security.md`](spec/11-security.md).

## Supported releases

NUIF has no stable release. The latest editor prerelease receives security
corrections when a report affects its declared profile. Earlier prereleases and
unpublished source revisions do not receive a separate support period. This
policy does not change the experimental status of the draft specification or
its conformance profiles.

## Private reporting

Report a suspected vulnerability through
[GitHub private vulnerability reporting](https://github.com/refpath/nuif/security/advisories/new).
Include the affected revision or release, input profile, reproduction steps,
observed result and expected boundary. Attach a minimized input when disclosure
of that input does not create additional risk.

Do not publish exploitable parser, renderer, installer or updater details in a
public issue before a coordinated correction is available. Non-sensitive bugs
and already-public dependency advisories may use the public issue tracker.

Maintainers triage reports in the private advisory, determine the affected
profiles and releases, prepare regression coverage and coordinate disclosure
with the reporter. A correction is not complete until its hostile-input or
regression case is exercised by the repository harness. Credit is recorded
when requested and when disclosure does not expose private information.

GitHub documents the private reporting and advisory workflow in *Privately
reporting a security vulnerability*, retrieved 2026-08-30:
https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/privately-reporting-a-security-vulnerability.
