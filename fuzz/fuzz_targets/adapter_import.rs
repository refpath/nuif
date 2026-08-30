#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let Some((&selector, bytes)) = input.split_first() else {
        return;
    };
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    match selector % 5 {
        0 => verify_html(source),
        1 => verify_svg(source),
        2 => verify_dtcg(source),
        3 => verify_react(source),
        _ => verify_svelte(source),
    }
});

fn verify_html(source: &str) {
    if let Ok(imported) = nuif_html::import_source(source) {
        let exported = nuif_html::export_document(&imported.document)
            .expect("accepted HTML profile document must export");
        let reimported =
            nuif_html::import_source(&exported.source).expect("exported HTML profile must import");
        assert_eq!(reimported.document, imported.document);
    }
}

fn verify_svg(source: &str) {
    if let Ok(imported) = nuif_svg::import_source(source) {
        let exported = nuif_svg::export_document(&imported.document)
            .expect("accepted SVG profile document must export");
        let reimported =
            nuif_svg::import_source(&exported.source).expect("exported SVG profile must import");
        assert_eq!(reimported.document, imported.document);
    }
}

fn verify_dtcg(source: &str) {
    if let Ok(imported) = nuif_dtcg::import_source(source) {
        let exported = nuif_dtcg::export_document(&imported.document)
            .expect("accepted DTCG profile document must export");
        let reimported =
            nuif_dtcg::import_source(&exported.source).expect("exported DTCG profile must import");
        assert_eq!(reimported.document, imported.document);
    }
}

fn verify_react(source: &str) {
    if let Ok(imported) = nuif_react::import_source(source) {
        let exported = nuif_react::export_document(&imported.document)
            .expect("accepted React profile document must export");
        let reimported = nuif_react::import_source(&exported.source)
            .expect("exported React profile must import");
        assert_eq!(reimported.document, imported.document);
    }
}

fn verify_svelte(source: &str) {
    if let Ok(imported) = nuif_svelte::import_source(source) {
        let exported = nuif_svelte::export_document(&imported.document)
            .expect("accepted Svelte profile document must export");
        let reimported = nuif_svelte::import_source(&exported.source)
            .expect("exported Svelte profile must import");
        assert_eq!(reimported.document, imported.document);
    }
}
