---
id: nuif:rfc:0012
kind: rfc
status: proposed
---

# RFC 0012 — Behavior as a content-addressed package resource

Status: proposed, with executable experimental profile
`nuif-behavior-package-resource-0`. This RFC chooses the first transport for
the bounded behavior experiment. It does not add behavior to the canonical
semantic `Document`, standardize a media type, or authorize execution when a
package is opened.

## Motivation

The independent Rust/Node behavior traces and the three-engine web lowering
show that one finite state-machine subset is executable. Keeping it only as an
out-of-band JSON fixture, however, cannot test offline delivery, package
identity, corruption handling or document-reference validation after a package
round trip. Moving it directly into the canonical document would make a much
larger and less-tested compatibility commitment.

The narrow requirement is therefore: transport one exact behavior program with
one exact document using the current deterministic package, while keeping
behavior modular and inert until an explicit capability-aware runtime accepts
it.

## Prior art and evidence

- `nuif:research:behavior-portability-state-machines` defines the finite trace
  contract and its current exclusions.
- `nuif:research:behavior-package-resource-binding` compares OCI descriptors,
  EPUB container processing and KHR_interactivity, then records the executable
  package decision.
- RFC 0010 already defines deterministic package manifests, digest-addressed
  blobs, resource roles, explicit resolution and the rule that resources do
  not execute merely because they are present.

## Profile

`nuif-behavior-package-resource-0` uses `nuif-package-0` unchanged. A conforming
attachment has all of these properties:

- exactly one resource descriptor has media type
  `application/nuif-behavior+cbor`;
- that media type is provisional and is not an IANA-registration claim;
- the descriptor role is `source`, its locator is embedded, and it has no
  derivation record;
- the embedded path is the normal `blobs/sha256/<digest>` path;
- the resource bytes are canonical `nuif-cbor-0` encoding of exactly one
  `nuif-behavior-state-machine-0` `BehaviorProgram`;
- `required_capabilities` contains `nuif-behavior-state-machine-0`;
- the complete program validates against `document.cbor` before attachment and
  after package decode.

Zero behavior resources with no behavior capability is a valid package without
behavior. A capability with no resource, a resource without its capability,
more than one behavior resource, a linked behavior resource, non-canonical
bytes or invalid document references fail the attachment profile.

## Identity and binding

The behavior resource digest identifies its exact canonical program bytes. The
document canonical hash identifies exact semantic document bytes. Neither is
redefined to include the other.

`manifest.cbor` carries both descriptors, and the deterministic complete
package hash binds their delivered pairing. Consequently:

- attaching behavior changes the package hash but not the document hash;
- changing the document changes both its hash and the complete package hash;
- the same behavior resource can be deduplicated by digest without claiming it
  is valid for every document;
- every explicit load revalidates stable entity references against the actual
  package document.

## Processing and authority

Package processing is layered:

1. `NuifPackage::decode` validates the deterministic ZIP, canonical manifest,
   document, descriptors, sizes, hashes and embedded bytes.
2. `attached_behavior` opts into this RFC, checks descriptor/capability
   agreement, decodes canonical behavior bytes and validates the program
   against the document.
3. A caller creates a behavior runtime only with an explicit set of available
   abstract-effect capabilities.
4. A host adapter separately maps admitted effects under its own profile and
   security policy.

Step 1 never implies steps 2–4. Generic tools may inspect, copy and deterministically
re-encode the package without executing the resource. Tools that claim full
behavioral conformance must understand the required behavior capability; a
structural package decode alone is not such a claim.

## Security and limits

The package verifies declared byte length and SHA-256 before canonical CBOR or
behavior validation. Existing package byte/resource/member limits and behavior
state/transition/action/string limits both apply. Linked behavior is rejected
so the profile neither invokes a resolver nor grants network authority.

The program remains data. It contains no scripts, dynamic imports, filesystem
paths, URLs, timers or host calls. Runtime creation and every host lowering are
separate authorization decisions.

## Conformance

`nuif:experiment:behavior-package-resource` requires:

- canonical program bytes and exact attach/decode/encode fixpoint;
- unchanged document hash and changed package hash after attachment;
- exact required capability, source role, media type and digest-derived path;
- refusal of missing/duplicate/linked/malformed/rebound resources;
- rejection of package-byte corruption before behavior decoding;
- an independent standard-library ZIP reader checking exact package bytes,
  member order/metadata, CRC and the content-addressed behavior resource.

`cargo xtask gate-behavior` includes this gate before the independent behavior
trace gate. CI archives all generated package and report artifacts.

## Rejected alternatives

- Canonical `Document.behavior` in the first experiment: premature semantic
  commitment and forced support in all readers.
- A special `behavior.cbor` ZIP path: duplicates the resource manifest and
  requires a new container path profile.
- An opaque extension inside an arbitrary entity: gives behavior no unique
  package-level discovery or cardinality rule and couples it to one visual
  node.
- JSON attachment: creates a competing canonical encoding and larger parsing
  surface.
- Automatic execution on package open: violates RFC 0010's inert-resource
  boundary and prevents safe inspection tools.

## Unresolved questions

- Which next event/effect types remain finite and portable across web, native
  and presentation hosts.
- When independently implemented adapters provide enough evidence to propose a
  canonical behavior model.
- Final media-type naming and registration timing.
- Generic SDK negotiation for packages containing multiple unrelated required
  capabilities.
