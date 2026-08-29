#![doc = "Executable profile-0 conformance assertions."]

#[cfg(test)]
mod tests {
    use nuif_api::{Session, profile_zero_context};
    use nuif_codec::{Decoder, DeterministicCbor, Encoder, canonical_hash};
    use nuif_core::{EntityId, EntityKind, FlowDirection, OpaquePayload};
    use nuif_html::{export_document, import_source, profile_fixture};
    use nuif_protocol::{Operation, Patch, Transaction, apply_patch};
    use nuif_testing::{TrialConfig, responsive_card_fixture, run_trials};

    #[test]
    fn responsive_card_changes_flow_at_declared_breakpoint() {
        let document = responsive_card_fixture();
        let narrow = Session::new(document.clone())
            .snapshot(&profile_zero_context(360.0, 640.0))
            .unwrap();
        let wide = Session::new(document)
            .snapshot(&profile_zero_context(768.0, 640.0))
            .unwrap();
        let media = EntityId::new(0x21);
        let copy = EntityId::new(0x22);
        assert!(narrow.layout.boxes[&copy].y > narrow.layout.boxes[&media].y);
        assert!(wide.layout.boxes[&copy].x > wide.layout.boxes[&media].x);
    }

    #[test]
    fn unknown_payload_survives_ignorant_neighbour_edit() {
        let base = responsive_card_fixture();
        let unknown_id = EntityId::new(0x25);
        let EntityKind::Unknown(before) = &base.entities[&unknown_id].kind else {
            panic!("fixture entity must be unknown");
        };
        let expected = before.payload.clone();
        let encoded = DeterministicCbor.encode(&base).unwrap();
        let mut ignorant = DeterministicCbor.decode(&encoded).unwrap();
        let patch = Patch {
            base_revision: canonical_hash(&ignorant).ok(),
            transactions: vec![Transaction {
                id: 1,
                operations: vec![Operation::Rename {
                    entity: EntityId::new(0x22),
                    name: Some("edited copy".to_owned()),
                }],
            }],
        };
        apply_patch(&mut ignorant, &patch).unwrap();
        let cycled = DeterministicCbor
            .decode(&DeterministicCbor.encode(&ignorant).unwrap())
            .unwrap();
        let EntityKind::Unknown(after) = &cycled.entities[&unknown_id].kind else {
            panic!("unknown kind was not restored");
        };
        assert_eq!(after.payload, expected);
        assert_eq!(
            cycled.entities[&unknown_id]
                .extensions
                .0
                .get("vendor.probe"),
            Some(&OpaquePayload {
                encoding: expected.encoding,
                bytes: expected.bytes.clone(),
            })
        );
    }

    #[test]
    fn seeded_roundtrip_trial_is_green() {
        let report = run_trials(&TrialConfig {
            seed: 0x5eed,
            iterations: 20,
            operations_per_iteration: 12,
            ..TrialConfig::default()
        });
        assert!(
            report.passed(),
            "{}",
            serde_json::to_string_pretty(&report).unwrap()
        );
    }

    #[test]
    fn fixture_declares_the_expected_responsive_rule() {
        let document = responsive_card_fixture();
        let card = &document.entities[&EntityId::new(0x20)];
        assert_eq!(card.authored.layout.direction, FlowDirection::Column);
        assert_eq!(
            card.authored.responsive[0].direction,
            Some(FlowDirection::Row)
        );
    }

    #[test]
    fn html_css_profile_roundtrip_is_exact_and_lossless() {
        let document = profile_fixture();
        let exported = export_document(&document).unwrap();
        let imported = import_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        assert!(exported.report.is_lossless());
        assert!(exported.report.unmapped_source_preserved);
        assert_eq!(
            exported.report.correspondences.len(),
            exported.report.fidelity.len()
        );
    }
}
