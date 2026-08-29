#![doc = "Encoding-independent codec contracts for NUIF."]

use nuif_core::Document;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingProfile {
    CanonicalTextV0,
    DeterministicCborV0,
}

pub trait Encoder {
    type Error;

    fn profile(&self) -> EncodingProfile;

    /// Encodes a semantic document using this encoder's declared profile.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when the document cannot be
    /// represented, validated, canonicalized, or written by the profile.
    fn encode(&self, document: &Document) -> Result<Vec<u8>, Self::Error>;
}

pub trait Decoder {
    type Error;

    fn profile(&self) -> EncodingProfile;

    /// Decodes bytes into a semantic document using this decoder's profile.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error for malformed, unsupported,
    /// non-conforming, or resource-limit-exceeding input.
    fn decode(&self, bytes: &[u8]) -> Result<Document, Self::Error>;
}

pub trait Canonicalizer {
    type Error;

    /// Rewrites an encoded document into the profile's canonical byte form.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when the input cannot be
    /// decoded or canonical output cannot be produced.
    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error>;
}
