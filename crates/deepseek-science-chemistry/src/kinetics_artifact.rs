//! Deterministic in-memory artifact preparation for completed kinetics analysis.

use crate::error::KineticsError;
use crate::kinetics::{
    artifact_review_status, KineticsAnalysisResult, CHEMISTRY_KINETICS_ARTIFACT_STEP,
    CHEMISTRY_KINETICS_CSV_WORKFLOW_ID, KINETICS_ARTIFACT_TITLE,
};
use deepseek_science_artifacts::{
    ArtifactContentDescriptor, ArtifactInputDescriptor, ArtifactKind, ArtifactProvenance,
    ArtifactReviewSummary, UnregisteredArtifactEnvelope, UnregisteredArtifactMetadata,
};

/// Schema version for the deterministic kinetics artifact envelope.
pub const KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION: &str = "kinetics.artifact.v1";

/// Schema version of the exact caller-provided kinetics analysis payload.
pub const KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION: &str = "kinetics.analysis.v1";

/// Role of the exact raw source bytes in a kinetics artifact envelope.
pub const KINETICS_ARTIFACT_SOURCE_ROLE: &str = "source_csv";

const JSON_MEDIA_TYPE: &str = "application/json";

/// Prepares an unregistered kinetics artifact envelope entirely in memory.
///
/// `analysis` must already be complete. The caller must supply `payload_utf8`
/// from the existing kinetics analysis JSON serializer in the same flow and
/// with the same input and column arguments. This adapter verifies exact
/// payload bytes and maps the existing review, but it cannot prove semantic
/// identity between arbitrary payload text and `analysis`. It performs no
/// parsing, analysis, review, outer serialization, persistence, or file IO.
pub fn prepare_kinetics_artifact_envelope(
    analysis: &KineticsAnalysisResult,
    raw_source_bytes: &[u8],
    payload_utf8: &str,
    producer_command: impl Into<String>,
    producer_version: impl Into<String>,
) -> Result<UnregisteredArtifactEnvelope, KineticsError> {
    let content = ArtifactContentDescriptor::from_utf8_payload(
        JSON_MEDIA_TYPE,
        KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION,
        payload_utf8,
    )?;
    let source_input =
        ArtifactInputDescriptor::from_bytes(KINETICS_ARTIFACT_SOURCE_ROLE, raw_source_bytes)?;
    let provenance = ArtifactProvenance::new(
        CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
        CHEMISTRY_KINETICS_ARTIFACT_STEP,
        producer_command,
        producer_version,
    )?;
    let finding_count = u64::try_from(analysis.review.findings.len())
        .map_err(|_| KineticsError::ArtifactReviewFindingCountOverflow)?;
    let review = ArtifactReviewSummary::new(
        artifact_review_status(analysis.review.status),
        finding_count,
    );
    let metadata = UnregisteredArtifactMetadata::new(
        ArtifactKind::Json,
        KINETICS_ARTIFACT_TITLE,
        content,
        vec![source_input],
        provenance,
        review,
    )?;

    UnregisteredArtifactEnvelope::new(
        KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION,
        metadata,
        payload_utf8,
    )
    .map_err(KineticsError::from)
}

#[cfg(test)]
mod tests {
    use super::{
        prepare_kinetics_artifact_envelope, KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION,
        KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION, KINETICS_ARTIFACT_SOURCE_ROLE,
    };
    use crate::{
        kinetics_csv_workflow_plan, KineticsAnalysisResult, KineticsArtifactProposal,
        KineticsColumns, KineticsError, KineticsReviewCheckKind, KineticsReviewFinding,
        KineticsReviewSeverity, KineticsReviewStatus, ValidatedKineticsInput,
        CHEMISTRY_KINETICS_ARTIFACT_STEP, CHEMISTRY_KINETICS_CSV_WORKFLOW_ID,
    };
    use deepseek_science_artifacts::{ArtifactError, ArtifactKind, ExactByteHash, ReviewStatus};
    use deepseek_science_common::{DataColumn, DataTable};

    const SAMPLE_PAYLOAD: &str =
        "{\"schema_version\":\"kinetics.analysis.v1\",\"command\":\"kinetics.analyze\"}\n";
    const SAMPLE_SOURCE: &[u8] = b"time,concentration\n0,1\n1,0.8\n";

    fn test_analysis() -> KineticsAnalysisResult {
        let table = DataTable::new(vec![
            DataColumn::numeric("time", vec![0.0, 1.0, 2.0]).expect("time column should construct"),
            DataColumn::numeric("concentration", vec![1.0, 0.8, 0.6])
                .expect("concentration column should construct"),
        ])
        .expect("table should construct");
        let columns =
            KineticsColumns::new("time", "concentration").expect("columns should construct");
        let input = ValidatedKineticsInput::from_table(&table, &columns)
            .expect("validated input should construct");

        KineticsAnalysisResult::analyze(&input).expect("analysis should succeed")
    }

    fn prepare(
        analysis: &KineticsAnalysisResult,
        source: &[u8],
        payload: &str,
    ) -> Result<deepseek_science_artifacts::UnregisteredArtifactEnvelope, KineticsError> {
        prepare_kinetics_artifact_envelope(analysis, source, payload, "example.command", "1.2.3")
    }

    fn existing_finding() -> KineticsReviewFinding {
        KineticsReviewFinding {
            severity: KineticsReviewSeverity::Warning,
            check_kind: KineticsReviewCheckKind::RejectedRowsVisible,
            model_kind: None,
            rejected_row_count: Some(1),
            message: "existing review finding",
        }
    }

    #[test]
    fn adapter_sets_frozen_kinetics_metadata() {
        let envelope = prepare(&test_analysis(), SAMPLE_SOURCE, SAMPLE_PAYLOAD)
            .expect("adapter should construct an envelope");
        let artifact = envelope.artifact();

        assert_eq!(
            KINETICS_ARTIFACT_ENVELOPE_SCHEMA_VERSION,
            "kinetics.artifact.v1"
        );
        assert_eq!(
            KINETICS_ANALYSIS_PAYLOAD_SCHEMA_VERSION,
            "kinetics.analysis.v1"
        );
        assert_eq!(KINETICS_ARTIFACT_SOURCE_ROLE, "source_csv");
        assert_eq!(envelope.schema_version(), "kinetics.artifact.v1");
        assert_eq!(artifact.kind(), ArtifactKind::Json);
        assert_eq!(artifact.title(), "Chemistry kinetics analysis result");
        assert_eq!(artifact.content().media_type(), "application/json");
        assert_eq!(artifact.content().schema_version(), "kinetics.analysis.v1");
        assert_eq!(artifact.content().encoding(), "utf-8");
        assert_eq!(artifact.inputs().len(), 1);
        assert_eq!(artifact.inputs()[0].role(), "source_csv");
        assert_eq!(
            artifact.provenance().workflow_id(),
            CHEMISTRY_KINETICS_CSV_WORKFLOW_ID
        );
        assert_eq!(
            artifact.provenance().workflow_step(),
            CHEMISTRY_KINETICS_ARTIFACT_STEP
        );
    }

    #[test]
    fn adapter_binds_exact_binary_source_bytes() {
        let source = [0xff, 0x00, 0x80, b'\n'];
        let envelope = prepare(&test_analysis(), &source, SAMPLE_PAYLOAD)
            .expect("binary source should be hashable");
        let input = &envelope.artifact().inputs()[0];

        assert_eq!(
            input.byte_length(),
            u64::try_from(source.len()).expect("test source length should fit")
        );
        assert_eq!(input.hash(), &ExactByteHash::blake3(&source));

        let changed = prepare(&test_analysis(), &[0xff, 0x00, 0x81, b'\n'], SAMPLE_PAYLOAD)
            .expect("changed source should be hashable");
        assert_ne!(input.hash(), changed.artifact().inputs()[0].hash());
    }

    #[test]
    fn adapter_binds_and_preserves_exact_payload_utf8() {
        let payload = "{\"label\":\"反应\"}\n";
        let envelope = prepare(&test_analysis(), SAMPLE_SOURCE, payload)
            .expect("non-ASCII payload should be accepted");
        let content = envelope.artifact().content();

        assert_eq!(envelope.payload_utf8(), payload);
        assert_eq!(
            content.byte_length(),
            u64::try_from(payload.len()).expect("test payload length should fit")
        );
        assert_ne!(
            content.byte_length(),
            u64::try_from(payload.chars().count()).expect("test character count should fit")
        );
        assert_eq!(content.hash(), &ExactByteHash::blake3(payload.as_bytes()));

        let changed_payload = "{\"label\":\"反应 changed\"}\n";
        let changed = prepare(&test_analysis(), SAMPLE_SOURCE, changed_payload)
            .expect("changed payload should be accepted");
        assert_ne!(content.hash(), changed.artifact().content().hash());
    }

    #[test]
    fn adapter_preserves_exact_producer_values() {
        let envelope = prepare_kinetics_artifact_envelope(
            &test_analysis(),
            SAMPLE_SOURCE,
            SAMPLE_PAYLOAD,
            " Example.Command ",
            " Version.Label ",
        )
        .expect("non-empty producer values should be accepted");
        let provenance = envelope.artifact().provenance();

        assert_eq!(provenance.producer_command(), " Example.Command ");
        assert_eq!(provenance.producer_version(), " Version.Label ");
    }

    #[test]
    fn adapter_propagates_blank_producer_errors() {
        let analysis = test_analysis();
        assert_eq!(
            prepare_kinetics_artifact_envelope(
                &analysis,
                SAMPLE_SOURCE,
                SAMPLE_PAYLOAD,
                " ",
                "1.2.3",
            ),
            Err(KineticsError::Artifact(ArtifactError::EmptyField {
                field: "producer_command"
            }))
        );
        assert_eq!(
            prepare_kinetics_artifact_envelope(
                &analysis,
                SAMPLE_SOURCE,
                SAMPLE_PAYLOAD,
                "example.command",
                "\t",
            ),
            Err(KineticsError::Artifact(ArtifactError::EmptyField {
                field: "producer_version"
            }))
        );
    }

    #[test]
    fn adapter_maps_existing_review_without_recomputing_it() {
        let cases = [
            (KineticsReviewStatus::Passed, ReviewStatus::Passed),
            (
                KineticsReviewStatus::PassedWithWarnings,
                ReviewStatus::PassedWithWarnings,
            ),
            (KineticsReviewStatus::Failed, ReviewStatus::Failed),
        ];

        for (kinetics_status, artifact_status) in cases {
            let mut analysis = test_analysis();
            analysis.review.status = kinetics_status;
            analysis.review.findings = vec![existing_finding()];
            let envelope = prepare(&analysis, SAMPLE_SOURCE, SAMPLE_PAYLOAD)
                .expect("existing review should map");

            assert_eq!(envelope.artifact().review().status(), artifact_status);
            assert_eq!(envelope.artifact().review().finding_count(), 1);
        }
    }

    #[test]
    fn adapter_propagates_payload_invariant_errors() {
        let analysis = test_analysis();
        let cases = [
            ("{}", ArtifactError::PayloadMissingFinalLf),
            ("{}\n\n", ArtifactError::PayloadHasMultipleTrailingLf),
            ("\u{feff}{}\n", ArtifactError::PayloadHasUtf8Bom),
            ("{\r}\n", ArtifactError::PayloadContainsCarriageReturn),
            ("", ArtifactError::EmptyPayload),
        ];

        for (payload, artifact_error) in cases {
            assert_eq!(
                prepare(&analysis, SAMPLE_SOURCE, payload),
                Err(KineticsError::Artifact(artifact_error))
            );
        }
    }

    #[test]
    fn adapter_is_deterministic_and_adds_no_runtime_metadata() {
        let analysis = test_analysis();
        let first = prepare(&analysis, SAMPLE_SOURCE, SAMPLE_PAYLOAD)
            .expect("first envelope should construct");
        let second = prepare(&analysis, SAMPLE_SOURCE, SAMPLE_PAYLOAD)
            .expect("second envelope should construct");
        let bytes = first
            .to_pretty_json_bytes_with_limit(4096)
            .expect("generic serializer should serialize the envelope");
        let text = std::str::from_utf8(&bytes).expect("serialized envelope should be UTF-8");

        assert_eq!(first, second);
        for prohibited in [
            "artifact_id",
            "run_id",
            "project_id",
            "timestamp",
            "path",
            "hostname",
            "username",
        ] {
            assert!(!text.contains(prohibited));
        }
    }

    #[test]
    fn envelope_hash_is_exact_payload_hash_not_legacy_semantic_hash() {
        let analysis = test_analysis();
        let proposal = KineticsArtifactProposal::from_analysis_result(&analysis)
            .expect("legacy proposal should construct");
        let envelope =
            prepare(&analysis, SAMPLE_SOURCE, SAMPLE_PAYLOAD).expect("envelope should construct");
        let exact_payload_hash = ExactByteHash::blake3(SAMPLE_PAYLOAD.as_bytes());

        assert_eq!(envelope.artifact().content().hash(), &exact_payload_hash);
        assert_ne!(
            envelope.artifact().content().hash().value(),
            proposal.content_hash
        );
        assert!(proposal.input_hashes.is_empty());
    }

    #[test]
    fn workflow_plan_contains_the_frozen_artifact_step() {
        let plan = kinetics_csv_workflow_plan().expect("workflow plan should construct");
        let keys: Vec<_> = plan.step_keys().map(|key| key.as_str()).collect();

        assert_eq!(keys[6], CHEMISTRY_KINETICS_ARTIFACT_STEP);
    }
}
