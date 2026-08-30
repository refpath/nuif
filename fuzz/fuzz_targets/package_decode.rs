#![no_main]

use libfuzzer_sys::fuzz_target;
use nuif_package::{MAX_PACKAGE_BYTES, NuifPackage};

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return;
    }
    if let Ok(package) = NuifPackage::decode(bytes) {
        let canonical = package.encode().expect("accepted package must encode");
        let decoded = NuifPackage::decode(&canonical).expect("encoded package must decode");
        let recanonical = decoded.encode().expect("decoded package must encode");
        assert_eq!(recanonical, canonical, "NUIF package byte fixpoint failed");
    }
    if let Ok(imported) = nuif_penpot::import_package(bytes) {
        let exported = nuif_penpot::export_document(&imported.document)
            .expect("accepted Penpot profile document must export");
        let reimported = nuif_penpot::import_package(&exported.bytes)
            .expect("exported Penpot package must import");
        assert_eq!(reimported.document, imported.document);
    }
});
