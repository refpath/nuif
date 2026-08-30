#![no_main]

use libfuzzer_sys::fuzz_target;
use nuif_codec::{CanonicalText, Canonicalizer, Decoder, DeterministicCbor, Encoder};
use nuif_core::{Severity, validate};

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > nuif_codec::MAX_INPUT_BYTES {
        return;
    }
    verify_codec(CanonicalText, bytes);
    verify_codec(DeterministicCbor, bytes);
});

fn verify_codec<C, E>(codec: C, bytes: &[u8])
where
    C: Copy + Decoder<Error = E> + Encoder<Error = E> + Canonicalizer<Error = E>,
    E: std::fmt::Debug,
{
    let Ok(document) = codec.decode(bytes) else {
        return;
    };
    assert!(
        validate(&document)
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error)
    );
    let canonical = codec
        .encode(&document)
        .expect("accepted document must encode");
    let recanonical = codec
        .canonicalize(&canonical)
        .expect("accepted document must canonicalize");
    assert_eq!(recanonical, canonical);
    let decoded = codec
        .decode(&canonical)
        .expect("canonical document must decode");
    assert_eq!(
        decoded, document,
        "canonical round trip changed the semantic document"
    );
}
