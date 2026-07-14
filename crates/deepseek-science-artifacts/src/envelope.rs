//! Deterministic in-memory envelopes for unregistered artifacts.

use crate::{ArtifactError, ArtifactKind, ExactByteHash, ReviewStatus};
use serde::Serialize;
use std::io::{self, Write};

const UTF8_ENCODING: &str = "utf-8";

/// Exact UTF-8 payload metadata for an unregistered artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactContentDescriptor {
    media_type: String,
    schema_version: String,
    encoding: String,
    byte_length: u64,
    hash: ExactByteHash,
}

impl ArtifactContentDescriptor {
    /// Describes the exact bytes of an already serialized UTF-8 payload.
    ///
    /// The payload is not parsed, normalized, or modified.
    pub fn from_utf8_payload(
        media_type: impl Into<String>,
        schema_version: impl Into<String>,
        payload_utf8: &str,
    ) -> Result<Self, ArtifactError> {
        let media_type = required_string(media_type, "media_type")?;
        let schema_version = required_string(schema_version, "schema_version")?;
        let bytes = payload_utf8.as_bytes();

        Ok(Self {
            media_type,
            schema_version,
            encoding: UTF8_ENCODING.to_string(),
            byte_length: byte_length(bytes)?,
            hash: ExactByteHash::blake3(bytes),
        })
    }

    /// Returns the caller-supplied media type unchanged.
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Returns the caller-supplied payload schema version unchanged.
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Returns the fixed UTF-8 encoding label.
    pub fn encoding(&self) -> &str {
        &self.encoding
    }

    /// Returns the exact UTF-8 payload byte length.
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the exact payload byte hash.
    pub fn hash(&self) -> &ExactByteHash {
        &self.hash
    }
}

/// Exact-byte descriptor for one artifact input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactInputDescriptor {
    role: String,
    byte_length: u64,
    hash: ExactByteHash,
}

impl ArtifactInputDescriptor {
    /// Describes the supplied raw bytes without decoding or normalization.
    pub fn from_bytes(role: impl Into<String>, bytes: &[u8]) -> Result<Self, ArtifactError> {
        Ok(Self {
            role: required_string(role, "role")?,
            byte_length: byte_length(bytes)?,
            hash: ExactByteHash::blake3(bytes),
        })
    }

    /// Returns the caller-supplied input role unchanged.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Returns the exact input byte length.
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the exact input byte hash.
    pub fn hash(&self) -> &ExactByteHash {
        &self.hash
    }
}

/// Domain-neutral provenance for one artifact-producing workflow step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactProvenance {
    workflow_id: String,
    workflow_step: String,
    producer_command: String,
    producer_version: String,
}

impl ArtifactProvenance {
    /// Creates provenance from four exact non-empty machine-readable values.
    pub fn new(
        workflow_id: impl Into<String>,
        workflow_step: impl Into<String>,
        producer_command: impl Into<String>,
        producer_version: impl Into<String>,
    ) -> Result<Self, ArtifactError> {
        Ok(Self {
            workflow_id: required_string(workflow_id, "workflow_id")?,
            workflow_step: required_string(workflow_step, "workflow_step")?,
            producer_command: required_string(producer_command, "producer_command")?,
            producer_version: required_string(producer_version, "producer_version")?,
        })
    }

    /// Returns the workflow identifier unchanged.
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// Returns the workflow step unchanged.
    pub fn workflow_step(&self) -> &str {
        &self.workflow_step
    }

    /// Returns the producer command unchanged.
    pub fn producer_command(&self) -> &str {
        &self.producer_command
    }

    /// Returns the producer version unchanged.
    pub fn producer_version(&self) -> &str {
        &self.producer_version
    }
}

/// Compact summary of an existing artifact review result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReviewSummary {
    status: ReviewStatus,
    finding_count: u64,
}

impl ArtifactReviewSummary {
    /// Creates a review summary without running a reviewer.
    pub fn new(status: ReviewStatus, finding_count: u64) -> Self {
        Self {
            status,
            finding_count,
        }
    }

    /// Returns the existing generic review status.
    pub fn status(&self) -> ReviewStatus {
        self.status
    }

    /// Returns the number of findings in the existing review.
    pub fn finding_count(&self) -> u64 {
        self.finding_count
    }
}

/// Domain-neutral metadata for an artifact that has not been registered.
///
/// This metadata has no runtime instance identity and carries no storage or
/// environment details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnregisteredArtifactMetadata {
    kind: ArtifactKind,
    title: String,
    content: ArtifactContentDescriptor,
    inputs: Vec<ArtifactInputDescriptor>,
    provenance: ArtifactProvenance,
    review: ArtifactReviewSummary,
}

impl UnregisteredArtifactMetadata {
    /// Creates checked metadata for one unregistered artifact.
    pub fn new(
        kind: ArtifactKind,
        title: impl Into<String>,
        content: ArtifactContentDescriptor,
        inputs: Vec<ArtifactInputDescriptor>,
        provenance: ArtifactProvenance,
        review: ArtifactReviewSummary,
    ) -> Result<Self, ArtifactError> {
        let title = required_string(title, "title")?;
        if inputs.is_empty() {
            return Err(ArtifactError::EmptyInputs);
        }

        Ok(Self {
            kind,
            title,
            content,
            inputs,
            provenance,
            review,
        })
    }

    /// Returns the generic artifact kind.
    pub fn kind(&self) -> ArtifactKind {
        self.kind
    }

    /// Returns the caller-supplied title unchanged.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the exact payload descriptor.
    pub fn content(&self) -> &ArtifactContentDescriptor {
        &self.content
    }

    /// Returns the ordered source input descriptors.
    pub fn inputs(&self) -> &[ArtifactInputDescriptor] {
        &self.inputs
    }

    /// Returns the domain-neutral producer provenance.
    pub fn provenance(&self) -> &ArtifactProvenance {
        &self.provenance
    }

    /// Returns the existing review summary.
    pub fn review(&self) -> &ArtifactReviewSummary {
        &self.review
    }
}

/// Deterministic in-memory envelope for an unregistered artifact.
///
/// The payload remains exact decoded UTF-8 text, including its final LF. The
/// envelope has no registered instance identity and performs no persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnregisteredArtifactEnvelope {
    schema_version: String,
    artifact: UnregisteredArtifactMetadata,
    payload_utf8: String,
}

impl UnregisteredArtifactEnvelope {
    /// Creates an envelope and verifies its exact payload descriptor.
    pub fn new(
        schema_version: impl Into<String>,
        artifact: UnregisteredArtifactMetadata,
        payload_utf8: impl Into<String>,
    ) -> Result<Self, ArtifactError> {
        let schema_version = required_string(schema_version, "schema_version")?;
        let payload_utf8 = payload_utf8.into();
        validate_payload_text(&payload_utf8)?;

        let payload_bytes = payload_utf8.as_bytes();
        let content = artifact.content();
        if content.encoding() != UTF8_ENCODING {
            return Err(ArtifactError::PayloadEncodingMismatch);
        }
        if content.byte_length() != byte_length(payload_bytes)? {
            return Err(ArtifactError::PayloadByteLengthMismatch);
        }
        if content.hash() != &ExactByteHash::blake3(payload_bytes) {
            return Err(ArtifactError::PayloadHashMismatch);
        }

        Ok(Self {
            schema_version,
            artifact,
            payload_utf8,
        })
    }

    /// Returns the caller-supplied envelope schema version unchanged.
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Returns the checked unregistered artifact metadata.
    pub fn artifact(&self) -> &UnregisteredArtifactMetadata {
        &self.artifact
    }

    /// Returns the exact decoded UTF-8 payload text.
    pub fn payload_utf8(&self) -> &str {
        &self.payload_utf8
    }

    /// Serializes deterministic pretty JSON within an inclusive byte limit.
    ///
    /// The returned UTF-8 bytes use declaration-ordered fields, two-space
    /// indentation, LF-only line endings, and exactly one final outer LF.
    pub fn to_pretty_json_bytes_with_limit(
        &self,
        max_bytes: usize,
    ) -> Result<Vec<u8>, ArtifactError> {
        let json_maximum = max_bytes
            .checked_sub(1)
            .ok_or(ArtifactError::SerializedEnvelopeTooLarge { maximum: max_bytes })?;
        let inputs = self
            .artifact
            .inputs()
            .iter()
            .map(|input| SerializedInput {
                role: input.role(),
                byte_length: input.byte_length(),
                hash: serialized_hash(input.hash()),
            })
            .collect();
        let serialized = SerializedEnvelope {
            schema_version: self.schema_version(),
            artifact: SerializedArtifact {
                kind: self.artifact.kind().machine_label(),
                title: self.artifact.title(),
                content: SerializedContent {
                    media_type: self.artifact.content().media_type(),
                    schema_version: self.artifact.content().schema_version(),
                    encoding: self.artifact.content().encoding(),
                    byte_length: self.artifact.content().byte_length(),
                    hash: serialized_hash(self.artifact.content().hash()),
                },
                inputs,
                provenance: SerializedProvenance {
                    workflow_id: self.artifact.provenance().workflow_id(),
                    workflow_step: self.artifact.provenance().workflow_step(),
                    producer_command: self.artifact.provenance().producer_command(),
                    producer_version: self.artifact.provenance().producer_version(),
                },
                review: SerializedReview {
                    status: self.artifact.review().status().machine_label(),
                    finding_count: self.artifact.review().finding_count(),
                },
            },
            payload_utf8: self.payload_utf8(),
        };
        let mut writer = BoundedMemoryWriter::new(json_maximum);

        if serde_json::to_writer_pretty(&mut writer, &serialized).is_err() {
            return if writer.limit_exceeded() {
                Err(ArtifactError::SerializedEnvelopeTooLarge { maximum: max_bytes })
            } else {
                Err(ArtifactError::SerializationFailed)
            };
        }

        writer.finish_with_lf(max_bytes)
    }
}

fn required_string(value: impl Into<String>, field: &'static str) -> Result<String, ArtifactError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(ArtifactError::EmptyField { field });
    }
    Ok(value)
}

fn byte_length(bytes: &[u8]) -> Result<u64, ArtifactError> {
    u64::try_from(bytes.len()).map_err(|_| ArtifactError::ByteLengthOverflow)
}

fn validate_payload_text(payload_utf8: &str) -> Result<(), ArtifactError> {
    if payload_utf8.is_empty() {
        return Err(ArtifactError::EmptyPayload);
    }
    if payload_utf8.starts_with('\u{feff}') {
        return Err(ArtifactError::PayloadHasUtf8Bom);
    }
    if payload_utf8.contains('\r') {
        return Err(ArtifactError::PayloadContainsCarriageReturn);
    }
    if !payload_utf8.ends_with('\n') {
        return Err(ArtifactError::PayloadMissingFinalLf);
    }
    if payload_utf8
        .strip_suffix('\n')
        .is_some_and(|without_final_lf| without_final_lf.ends_with('\n'))
    {
        return Err(ArtifactError::PayloadHasMultipleTrailingLf);
    }
    Ok(())
}

#[derive(Serialize)]
struct SerializedEnvelope<'a> {
    schema_version: &'a str,
    artifact: SerializedArtifact<'a>,
    payload_utf8: &'a str,
}

#[derive(Serialize)]
struct SerializedArtifact<'a> {
    kind: &'static str,
    title: &'a str,
    content: SerializedContent<'a>,
    inputs: Vec<SerializedInput<'a>>,
    provenance: SerializedProvenance<'a>,
    review: SerializedReview,
}

#[derive(Serialize)]
struct SerializedContent<'a> {
    media_type: &'a str,
    schema_version: &'a str,
    encoding: &'a str,
    byte_length: u64,
    hash: SerializedHash<'a>,
}

#[derive(Serialize)]
struct SerializedInput<'a> {
    role: &'a str,
    byte_length: u64,
    hash: SerializedHash<'a>,
}

#[derive(Serialize)]
struct SerializedHash<'a> {
    algorithm: &'static str,
    value: &'a str,
}

#[derive(Serialize)]
struct SerializedProvenance<'a> {
    workflow_id: &'a str,
    workflow_step: &'a str,
    producer_command: &'a str,
    producer_version: &'a str,
}

#[derive(Serialize)]
struct SerializedReview {
    status: &'static str,
    finding_count: u64,
}

fn serialized_hash(hash: &ExactByteHash) -> SerializedHash<'_> {
    SerializedHash {
        algorithm: hash.algorithm().machine_label(),
        value: hash.value(),
    }
}

struct BoundedMemoryWriter {
    bytes: Vec<u8>,
    maximum: usize,
    limit_exceeded: bool,
}

impl BoundedMemoryWriter {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
            limit_exceeded: false,
        }
    }

    fn limit_exceeded(&self) -> bool {
        self.limit_exceeded
    }

    fn finish_with_lf(mut self, max_bytes: usize) -> Result<Vec<u8>, ArtifactError> {
        let final_length = self
            .bytes
            .len()
            .checked_add(1)
            .ok_or(ArtifactError::SerializedEnvelopeTooLarge { maximum: max_bytes })?;
        if final_length > max_bytes {
            return Err(ArtifactError::SerializedEnvelopeTooLarge { maximum: max_bytes });
        }

        self.bytes.push(b'\n');
        Ok(self.bytes)
    }
}

impl Write for BoundedMemoryWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let Some(new_length) = self.bytes.len().checked_add(buffer.len()) else {
            self.limit_exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "bounded artifact envelope buffer exceeded",
            ));
        };
        if new_length > self.maximum {
            self.limit_exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "bounded artifact envelope buffer exceeded",
            ));
        }

        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactContentDescriptor, ArtifactInputDescriptor, ArtifactProvenance,
        ArtifactReviewSummary, UnregisteredArtifactEnvelope, UnregisteredArtifactMetadata,
    };
    use crate::{hash_bytes, ArtifactError, ArtifactKind, ExactByteHash, ReviewStatus};

    const SAMPLE_PAYLOAD: &str = "{\"value\":\"λ\"}\n";
    const SAMPLE_INPUT: &[u8] = &[0, 255, 10];

    fn sample_content() -> ArtifactContentDescriptor {
        ArtifactContentDescriptor::from_utf8_payload(
            "application/json",
            "payload.result.v1",
            SAMPLE_PAYLOAD,
        )
        .expect("sample content should be valid")
    }

    fn sample_input() -> ArtifactInputDescriptor {
        ArtifactInputDescriptor::from_bytes("primary_input", SAMPLE_INPUT)
            .expect("sample input should be valid")
    }

    fn sample_provenance() -> ArtifactProvenance {
        ArtifactProvenance::new(
            "example.workflow",
            "produce_result",
            "example.command",
            "1.2.3",
        )
        .expect("sample provenance should be valid")
    }

    fn sample_metadata_with_content(
        content: ArtifactContentDescriptor,
    ) -> UnregisteredArtifactMetadata {
        UnregisteredArtifactMetadata::new(
            ArtifactKind::Json,
            "Example result",
            content,
            vec![sample_input()],
            sample_provenance(),
            ArtifactReviewSummary::new(ReviewStatus::Passed, 0),
        )
        .expect("sample metadata should be valid")
    }

    fn sample_metadata() -> UnregisteredArtifactMetadata {
        sample_metadata_with_content(sample_content())
    }

    fn sample_envelope() -> UnregisteredArtifactEnvelope {
        UnregisteredArtifactEnvelope::new("artifact.envelope.v1", sample_metadata(), SAMPLE_PAYLOAD)
            .expect("sample envelope should be valid")
    }

    #[test]
    fn content_descriptor_uses_exact_utf8_bytes() {
        let payload = "λ\n";
        let descriptor =
            ArtifactContentDescriptor::from_utf8_payload("application/json", "payload.v1", payload)
                .expect("content descriptor should construct");

        assert_eq!(descriptor.media_type(), "application/json");
        assert_eq!(descriptor.schema_version(), "payload.v1");
        assert_eq!(descriptor.encoding(), "utf-8");
        assert_eq!(descriptor.byte_length(), 3);
        assert_ne!(descriptor.byte_length(), payload.chars().count() as u64);
        assert_eq!(descriptor.hash().value(), hash_bytes(payload.as_bytes()));
        assert_ne!(descriptor.hash().value(), hash_bytes("λ".as_bytes()));
    }

    #[test]
    fn content_descriptor_rejects_empty_labels() {
        assert_eq!(
            ArtifactContentDescriptor::from_utf8_payload(" ", "payload.v1", "{}\n"),
            Err(ArtifactError::EmptyField {
                field: "media_type"
            })
        );
        assert_eq!(
            ArtifactContentDescriptor::from_utf8_payload("application/json", "\t", "{}\n"),
            Err(ArtifactError::EmptyField {
                field: "schema_version"
            })
        );
    }

    #[test]
    fn content_descriptor_preserves_exact_labels_and_payload_hash() {
        let descriptor = ArtifactContentDescriptor::from_utf8_payload(
            " application/json ",
            " Payload.V1 ",
            SAMPLE_PAYLOAD,
        )
        .expect("non-empty exact labels should be accepted");

        assert_eq!(descriptor.media_type(), " application/json ");
        assert_eq!(descriptor.schema_version(), " Payload.V1 ");
        assert_eq!(
            descriptor.hash(),
            &ExactByteHash::blake3(SAMPLE_PAYLOAD.as_bytes())
        );
    }

    #[test]
    fn input_descriptor_hashes_raw_binary_bytes() {
        let bytes = [0xff, 0x00, 0x80, b'\n'];
        let descriptor = ArtifactInputDescriptor::from_bytes(" binary_input ", &bytes)
            .expect("binary input should be accepted");

        assert_eq!(descriptor.role(), " binary_input ");
        assert_eq!(descriptor.byte_length(), bytes.len() as u64);
        assert_eq!(descriptor.hash().value(), hash_bytes(&bytes));
    }

    #[test]
    fn input_descriptor_rejects_empty_role() {
        assert_eq!(
            ArtifactInputDescriptor::from_bytes("\n", b"input"),
            Err(ArtifactError::EmptyField { field: "role" })
        );
    }

    #[test]
    fn provenance_preserves_exact_values() {
        let provenance = ArtifactProvenance::new(
            " Workflow.ID ",
            " Step.Name ",
            " Command.Name ",
            " Version.Label ",
        )
        .expect("non-empty exact values should be accepted");

        assert_eq!(provenance.workflow_id(), " Workflow.ID ");
        assert_eq!(provenance.workflow_step(), " Step.Name ");
        assert_eq!(provenance.producer_command(), " Command.Name ");
        assert_eq!(provenance.producer_version(), " Version.Label ");
    }

    #[test]
    fn provenance_rejects_each_trim_empty_field() {
        assert_eq!(
            ArtifactProvenance::new(" ", "step", "command", "1"),
            Err(ArtifactError::EmptyField {
                field: "workflow_id"
            })
        );
        assert_eq!(
            ArtifactProvenance::new("workflow", "\t", "command", "1"),
            Err(ArtifactError::EmptyField {
                field: "workflow_step"
            })
        );
        assert_eq!(
            ArtifactProvenance::new("workflow", "step", "\n", "1"),
            Err(ArtifactError::EmptyField {
                field: "producer_command"
            })
        );
        assert_eq!(
            ArtifactProvenance::new("workflow", "step", "command", "  "),
            Err(ArtifactError::EmptyField {
                field: "producer_version"
            })
        );
    }

    #[test]
    fn review_summary_preserves_existing_status_and_count() {
        let summary = ArtifactReviewSummary::new(ReviewStatus::NotReviewed, 7);

        assert_eq!(summary.status(), ReviewStatus::NotReviewed);
        assert_eq!(summary.finding_count(), 7);
    }

    #[test]
    fn metadata_preserves_generic_fields_and_inputs() {
        let metadata = sample_metadata();

        assert_eq!(metadata.kind(), ArtifactKind::Json);
        assert_eq!(metadata.title(), "Example result");
        assert_eq!(metadata.content(), &sample_content());
        assert_eq!(metadata.inputs(), &[sample_input()]);
        assert_eq!(metadata.provenance(), &sample_provenance());
        assert_eq!(
            metadata.review(),
            &ArtifactReviewSummary::new(ReviewStatus::Passed, 0)
        );
    }

    #[test]
    fn metadata_rejects_empty_title_and_empty_inputs() {
        assert_eq!(
            UnregisteredArtifactMetadata::new(
                ArtifactKind::Text,
                " ",
                sample_content(),
                vec![sample_input()],
                sample_provenance(),
                ArtifactReviewSummary::new(ReviewStatus::NotReviewed, 0),
            ),
            Err(ArtifactError::EmptyField { field: "title" })
        );
        assert_eq!(
            UnregisteredArtifactMetadata::new(
                ArtifactKind::Text,
                "Title",
                sample_content(),
                Vec::new(),
                sample_provenance(),
                ArtifactReviewSummary::new(ReviewStatus::NotReviewed, 0),
            ),
            Err(ArtifactError::EmptyInputs)
        );
    }

    #[test]
    fn metadata_preserves_multiple_inputs_in_caller_order() {
        let first = ArtifactInputDescriptor::from_bytes("first", b"one")
            .expect("first input should construct");
        let second = ArtifactInputDescriptor::from_bytes("second", b"two")
            .expect("second input should construct");
        let metadata = UnregisteredArtifactMetadata::new(
            ArtifactKind::Json,
            "Ordered inputs",
            sample_content(),
            vec![first.clone(), second.clone()],
            sample_provenance(),
            ArtifactReviewSummary::new(ReviewStatus::NotReviewed, 0),
        )
        .expect("multiple inputs should be accepted");

        assert_eq!(metadata.inputs(), &[first, second]);
    }

    #[test]
    fn envelope_rejects_empty_schema_and_payload() {
        assert_eq!(
            UnregisteredArtifactEnvelope::new(" ", sample_metadata(), SAMPLE_PAYLOAD),
            Err(ArtifactError::EmptyField {
                field: "schema_version"
            })
        );
        assert_eq!(
            UnregisteredArtifactEnvelope::new(
                "artifact.envelope.v1",
                sample_metadata_with_content(
                    ArtifactContentDescriptor::from_utf8_payload(
                        "application/json",
                        "payload.result.v1",
                        "",
                    )
                    .expect("empty payload can be described before envelope validation"),
                ),
                "",
            ),
            Err(ArtifactError::EmptyPayload)
        );
    }

    #[test]
    fn envelope_rejects_payload_bom() {
        let payload = "\u{feff}{}\n";
        let metadata = sample_metadata_with_content(
            ArtifactContentDescriptor::from_utf8_payload(
                "application/json",
                "payload.result.v1",
                payload,
            )
            .expect("descriptor should preserve bytes"),
        );

        assert_eq!(
            UnregisteredArtifactEnvelope::new("artifact.envelope.v1", metadata, payload),
            Err(ArtifactError::PayloadHasUtf8Bom)
        );
    }

    #[test]
    fn envelope_rejects_payload_carriage_return() {
        let payload = "{\r}\n";
        let metadata = sample_metadata_with_content(
            ArtifactContentDescriptor::from_utf8_payload(
                "application/json",
                "payload.result.v1",
                payload,
            )
            .expect("descriptor should preserve bytes"),
        );

        assert_eq!(
            UnregisteredArtifactEnvelope::new("artifact.envelope.v1", metadata, payload),
            Err(ArtifactError::PayloadContainsCarriageReturn)
        );
    }

    #[test]
    fn envelope_requires_exactly_one_trailing_lf() {
        let missing = "{}";
        let missing_metadata = sample_metadata_with_content(
            ArtifactContentDescriptor::from_utf8_payload(
                "application/json",
                "payload.result.v1",
                missing,
            )
            .expect("descriptor should preserve bytes"),
        );
        assert_eq!(
            UnregisteredArtifactEnvelope::new("artifact.envelope.v1", missing_metadata, missing,),
            Err(ArtifactError::PayloadMissingFinalLf)
        );

        let multiple = "{}\n\n";
        let multiple_metadata = sample_metadata_with_content(
            ArtifactContentDescriptor::from_utf8_payload(
                "application/json",
                "payload.result.v1",
                multiple,
            )
            .expect("descriptor should preserve bytes"),
        );
        assert_eq!(
            UnregisteredArtifactEnvelope::new("artifact.envelope.v1", multiple_metadata, multiple,),
            Err(ArtifactError::PayloadHasMultipleTrailingLf)
        );
    }

    #[test]
    fn envelope_rejects_content_encoding_mismatch() {
        let mut content = sample_content();
        content.encoding = "utf-16".to_string();

        assert_eq!(
            UnregisteredArtifactEnvelope::new(
                "artifact.envelope.v1",
                sample_metadata_with_content(content),
                SAMPLE_PAYLOAD,
            ),
            Err(ArtifactError::PayloadEncodingMismatch)
        );
    }

    #[test]
    fn envelope_rejects_content_length_mismatch() {
        let mut content = sample_content();
        content.byte_length += 1;

        assert_eq!(
            UnregisteredArtifactEnvelope::new(
                "artifact.envelope.v1",
                sample_metadata_with_content(content),
                SAMPLE_PAYLOAD,
            ),
            Err(ArtifactError::PayloadByteLengthMismatch)
        );
    }

    #[test]
    fn envelope_rejects_content_hash_mismatch() {
        let mut content = sample_content();
        content.hash = ExactByteHash::blake3(b"different payload\n");

        assert_eq!(
            UnregisteredArtifactEnvelope::new(
                "artifact.envelope.v1",
                sample_metadata_with_content(content),
                SAMPLE_PAYLOAD,
            ),
            Err(ArtifactError::PayloadHashMismatch)
        );
    }

    #[test]
    fn valid_envelope_preserves_exact_payload_and_metadata() {
        let envelope = sample_envelope();

        assert_eq!(envelope.schema_version(), "artifact.envelope.v1");
        assert_eq!(envelope.artifact(), &sample_metadata());
        assert_eq!(envelope.payload_utf8(), SAMPLE_PAYLOAD);
    }

    #[test]
    fn serialization_is_byte_identical_and_matches_normative_format() {
        let envelope = sample_envelope();
        let first = envelope
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit");
        let second = envelope
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit again");
        let expected = r#"{
  "schema_version": "artifact.envelope.v1",
  "artifact": {
    "kind": "json",
    "title": "Example result",
    "content": {
      "media_type": "application/json",
      "schema_version": "payload.result.v1",
      "encoding": "utf-8",
      "byte_length": <PAYLOAD_LENGTH>,
      "hash": {
        "algorithm": "blake3",
        "value": "<PAYLOAD_HASH>"
      }
    },
    "inputs": [
      {
        "role": "primary_input",
        "byte_length": 3,
        "hash": {
          "algorithm": "blake3",
          "value": "<INPUT_HASH>"
        }
      }
    ],
    "provenance": {
      "workflow_id": "example.workflow",
      "workflow_step": "produce_result",
      "producer_command": "example.command",
      "producer_version": "1.2.3"
    },
    "review": {
      "status": "passed",
      "finding_count": 0
    }
  },
  "payload_utf8": "{\"value\":\"λ\"}\n"
}
"#
        .replace(
            "<PAYLOAD_LENGTH>",
            SAMPLE_PAYLOAD.len().to_string().as_str(),
        )
        .replace(
            "<PAYLOAD_HASH>",
            hash_bytes(SAMPLE_PAYLOAD.as_bytes()).as_str(),
        )
        .replace("<INPUT_HASH>", hash_bytes(SAMPLE_INPUT).as_str());

        assert_eq!(first, second);
        assert_eq!(first, expected.as_bytes());
    }

    #[test]
    fn serialization_is_valid_json_and_decodes_exact_payload() {
        let bytes = sample_envelope()
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit");
        let decoded: serde_json::Value =
            serde_json::from_slice(&bytes).expect("envelope should be valid JSON");

        assert_eq!(decoded["schema_version"], "artifact.envelope.v1");
        assert_eq!(decoded["payload_utf8"].as_str(), Some(SAMPLE_PAYLOAD));
        assert!(decoded["payload_utf8"]
            .as_str()
            .expect("payload should be text")
            .ends_with('\n'));
    }

    #[test]
    fn serialized_bytes_follow_outer_byte_contract() {
        let bytes = sample_envelope()
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit");
        let text = std::str::from_utf8(&bytes).expect("output should be UTF-8");

        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(!bytes.contains(&b'\r'));
        assert!(text.lines().all(|line| !line.ends_with(' ')));
        assert!(bytes.ends_with(b"}\n"));
        assert!(!bytes.ends_with(b"}\n\n"));
        assert!(text.contains("\n  \"schema_version\""));
        assert!(text.contains("\n    \"kind\""));
    }

    #[test]
    fn serialization_enforces_exact_limit_without_returning_partial_bytes() {
        let envelope = sample_envelope();
        let bytes = envelope
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit");
        let exact = envelope
            .to_pretty_json_bytes_with_limit(bytes.len())
            .expect("exact limit should succeed");

        assert_eq!(exact, bytes);
        assert_eq!(
            envelope.to_pretty_json_bytes_with_limit(bytes.len() - 1),
            Err(ArtifactError::SerializedEnvelopeTooLarge {
                maximum: bytes.len() - 1
            })
        );
        assert_eq!(
            envelope.to_pretty_json_bytes_with_limit(0),
            Err(ArtifactError::SerializedEnvelopeTooLarge { maximum: 0 })
        );
        assert_eq!(
            envelope.to_pretty_json_bytes_with_limit(1),
            Err(ArtifactError::SerializedEnvelopeTooLarge { maximum: 1 })
        );
    }

    #[test]
    fn serialized_schema_has_no_runtime_identity_or_extra_metadata() {
        let bytes = sample_envelope()
            .to_pretty_json_bytes_with_limit(4096)
            .expect("sample should fit");
        let text = std::str::from_utf8(&bytes).expect("output should be UTF-8");
        let decoded: serde_json::Value =
            serde_json::from_slice(&bytes).expect("envelope should be valid JSON");
        let top = decoded.as_object().expect("top level should be an object");
        let artifact = decoded["artifact"]
            .as_object()
            .expect("artifact should be an object");

        assert_eq!(top.len(), 3);
        assert_eq!(artifact.len(), 6);
        for prohibited in [
            "artifact_id",
            "run_id",
            "project_id",
            "timestamp",
            "path",
            "model",
            "retrieval",
        ] {
            assert!(!text.contains(prohibited));
        }
    }
}
