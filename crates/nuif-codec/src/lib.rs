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
    fn encode(&self, document: &Document) -> Result<Vec<u8>, Self::Error>;
}

pub trait Decoder {
    type Error;

    fn profile(&self) -> EncodingProfile;
    fn decode(&self, bytes: &[u8]) -> Result<Document, Self::Error>;
}

pub trait Canonicalizer {
    type Error;

    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error>;
}
