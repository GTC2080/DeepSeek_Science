//! Pure in-memory plot data for deterministic kinetics visualization.
//!
//! This module prepares chemistry-owned observations and model predictions. It
//! does not parse CSV, refit models, render SVG, or perform file IO.

use thiserror::Error;

use crate::kinetics::{
    KineticsAnalysisResult, KineticsColumns, KineticsComparisonBasis, KineticsFitResult,
    KineticsModelKind, KineticsPoint, KineticsReviewStatus, ValidatedKineticsInput,
};

const CURVE_CANDIDATE_COUNT: usize = 128;
const LAST_CURVE_CANDIDATE_INDEX: usize = CURVE_CANDIDATE_COUNT - 1;

/// Errors raised while constructing deterministic kinetics plot data.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum KineticsPlotDataError {
    /// No accepted observations were available for a plot domain.
    #[error("kinetics plot data requires at least one accepted observation")]
    NoAcceptedPoints,

    /// Accepted counts disagree between validated input and analysis.
    #[error(
        "kinetics plot accepted count mismatch: validated={validated_count}, analysis={analysis_count}"
    )]
    AcceptedCountMismatch {
        /// Accepted count retained by validation.
        validated_count: usize,
        /// Accepted count reported by analysis.
        analysis_count: usize,
    },

    /// Rejected counts disagree between validated input and analysis.
    #[error(
        "kinetics plot rejected count mismatch: validated={validated_count}, analysis={analysis_count}"
    )]
    RejectedCountMismatch {
        /// Rejected count retained by validation.
        validated_count: usize,
        /// Rejected count reported by analysis.
        analysis_count: usize,
    },

    /// The review's rejected count disagrees with validated input.
    #[error(
        "kinetics plot review rejected count mismatch: validated={validated_count}, review={review_count}"
    )]
    ReviewRejectedCountMismatch {
        /// Rejected count retained by validation.
        validated_count: usize,
        /// Rejected count retained by deterministic review.
        review_count: usize,
    },

    /// A model fit reports a point count different from accepted observations.
    #[error(
        "kinetics plot fit point count mismatch for {model_kind:?}: accepted={accepted_count}, fit={fit_count}"
    )]
    FitPointCountMismatch {
        /// Model whose fit count disagrees.
        model_kind: KineticsModelKind,
        /// Accepted observation count.
        accepted_count: usize,
        /// Point count reported by the fit.
        fit_count: usize,
    },

    /// An accepted observation has a non-finite time.
    #[error("kinetics plot accepted time values must be finite")]
    NonFiniteAcceptedTime,

    /// Finite accepted time endpoints produced a non-finite span.
    #[error("kinetics plot accepted time span is not finite")]
    NonFiniteAcceptedTimeSpan,

    /// Accepted observations do not cover a positive time span.
    #[error("kinetics plot accepted time values must span a positive range")]
    NonPositiveAcceptedTimeSpan,

    /// Fit metadata required for prediction is non-finite.
    #[error("kinetics plot fit metadata is non-finite for {model_kind:?}")]
    NonFiniteFitMetadata {
        /// Model whose metadata is non-finite.
        model_kind: KineticsModelKind,
    },
}

/// One deterministic model prediction in the original concentration-time domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticsPredictionPoint {
    time: f64,
    concentration: f64,
}

impl KineticsPredictionPoint {
    /// Returns the sampled time.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Returns the predicted concentration.
    pub fn concentration(&self) -> f64 {
        self.concentration
    }
}

/// One contiguous, directly renderable sequence of valid model predictions.
#[derive(Clone, Debug, PartialEq)]
pub struct KineticsCurveSegment {
    points: Vec<KineticsPredictionPoint>,
}

impl KineticsCurveSegment {
    /// Returns predictions in ascending sampled-time order.
    pub fn points(&self) -> &[KineticsPredictionPoint] {
        &self.points
    }
}

/// Fit metadata and renderable curve segments for one kinetics model.
#[derive(Clone, Debug, PartialEq)]
pub struct KineticsPlotModelData {
    model_kind: KineticsModelKind,
    slope: f64,
    intercept: f64,
    r_squared: f64,
    segments: Vec<KineticsCurveSegment>,
}

impl KineticsPlotModelData {
    /// Returns the kinetics model represented.
    pub fn model_kind(&self) -> KineticsModelKind {
        self.model_kind
    }

    /// Returns the existing linearized fit slope.
    pub fn slope(&self) -> f64 {
        self.slope
    }

    /// Returns the existing linearized fit intercept.
    pub fn intercept(&self) -> f64 {
        self.intercept
    }

    /// Returns the existing linearized fit coefficient of determination.
    pub fn r_squared(&self) -> f64 {
        self.r_squared
    }

    /// Returns contiguous segments containing at least two predictions each.
    pub fn segments(&self) -> &[KineticsCurveSegment] {
        &self.segments
    }
}

/// Fixed visualization warnings produced by chemistry plot-data preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KineticsVisualizationWarning {
    /// Kinetics validation rejected one or more input rows.
    RejectedRowsPresent,
    /// Some first-order predictions were invalid, but a segment remains.
    FirstOrderPartiallyOmitted,
    /// Fewer than two finite first-order predictions remain.
    FirstOrderOmittedFewerThanTwoFinitePredictions,
    /// Some second-order predictions were invalid, but a segment remains.
    SecondOrderPartiallyOmitted,
    /// Fewer than two finite second-order predictions remain.
    SecondOrderOmittedFewerThanTwoFinitePredictions,
}

/// Immutable chemistry-owned data for a future kinetics visualization.
#[derive(Clone, Debug, PartialEq)]
pub struct KineticsPlotData {
    time_column: String,
    concentration_column: String,
    observations: Vec<KineticsPoint>,
    accepted_count: usize,
    rejected_count: usize,
    first_order: KineticsPlotModelData,
    second_order: KineticsPlotModelData,
    preferred_model: KineticsModelKind,
    comparison_basis: KineticsComparisonBasis,
    review_status: KineticsReviewStatus,
    review_finding_count: usize,
    warnings: Vec<KineticsVisualizationWarning>,
}

impl KineticsPlotData {
    /// Constructs plot data from one caller-maintained analysis flow.
    ///
    /// The caller must supply validated input, exact columns, and analysis from
    /// the same parse, validation, and analysis flow. Available count checks
    /// reject obvious structural mismatches but do not authenticate provenance
    /// or object identity. This constructor does not validate rows or refit.
    pub fn from_analysis(
        input: &ValidatedKineticsInput,
        columns: &KineticsColumns,
        analysis: &KineticsAnalysisResult,
    ) -> Result<Self, KineticsPlotDataError> {
        let accepted_count = input.valid_count();
        if accepted_count == 0 {
            return Err(KineticsPlotDataError::NoAcceptedPoints);
        }
        if accepted_count != analysis.valid_point_count() {
            return Err(KineticsPlotDataError::AcceptedCountMismatch {
                validated_count: accepted_count,
                analysis_count: analysis.valid_point_count(),
            });
        }

        let rejected_count = input.rejected_count();
        if rejected_count != analysis.rejected_row_count() {
            return Err(KineticsPlotDataError::RejectedCountMismatch {
                validated_count: rejected_count,
                analysis_count: analysis.rejected_row_count(),
            });
        }
        if rejected_count != analysis.review.rejected_row_count {
            return Err(KineticsPlotDataError::ReviewRejectedCountMismatch {
                validated_count: rejected_count,
                review_count: analysis.review.rejected_row_count,
            });
        }

        check_fit_point_count(analysis.comparison.first_order, accepted_count)?;
        check_fit_point_count(analysis.comparison.second_order, accepted_count)?;
        check_fit_metadata(analysis.comparison.first_order)?;
        check_fit_metadata(analysis.comparison.second_order)?;

        let (minimum_time, maximum_time) = accepted_time_domain(input.valid_points())?;
        let (first_order, first_warning) =
            build_model_data(analysis.comparison.first_order, minimum_time, maximum_time);
        let (second_order, second_warning) =
            build_model_data(analysis.comparison.second_order, minimum_time, maximum_time);

        let mut warnings = Vec::with_capacity(3);
        if rejected_count > 0 {
            warnings.push(KineticsVisualizationWarning::RejectedRowsPresent);
        }
        if let Some(warning) = first_warning {
            warnings.push(warning);
        }
        if let Some(warning) = second_warning {
            warnings.push(warning);
        }

        Ok(Self {
            time_column: columns.time().as_str().to_owned(),
            concentration_column: columns.concentration().as_str().to_owned(),
            observations: input.valid_points().to_vec(),
            accepted_count,
            rejected_count,
            first_order,
            second_order,
            preferred_model: analysis.preferred_model(),
            comparison_basis: analysis.comparison_basis(),
            review_status: analysis.review_status(),
            review_finding_count: analysis.review.findings.len(),
            warnings,
        })
    }

    /// Returns the exact caller-selected time column name.
    pub fn time_column(&self) -> &str {
        &self.time_column
    }

    /// Returns the exact caller-selected concentration column name.
    pub fn concentration_column(&self) -> &str {
        &self.concentration_column
    }

    /// Returns accepted observations in validation's caller-row order.
    pub fn observations(&self) -> &[KineticsPoint] {
        &self.observations
    }

    /// Returns the accepted observation count.
    pub fn accepted_count(&self) -> usize {
        self.accepted_count
    }

    /// Returns the rejected row count without rejected row contents.
    pub fn rejected_count(&self) -> usize {
        self.rejected_count
    }

    /// Returns first-order fit metadata and renderable predictions.
    pub fn first_order(&self) -> &KineticsPlotModelData {
        &self.first_order
    }

    /// Returns second-order fit metadata and renderable predictions.
    pub fn second_order(&self) -> &KineticsPlotModelData {
        &self.second_order
    }

    /// Returns the existing MVP heuristic preference unchanged.
    pub fn preferred_model(&self) -> KineticsModelKind {
        self.preferred_model
    }

    /// Returns the existing model-comparison basis unchanged.
    pub fn comparison_basis(&self) -> KineticsComparisonBasis {
        self.comparison_basis
    }

    /// Returns the existing deterministic review status unchanged.
    pub fn review_status(&self) -> KineticsReviewStatus {
        self.review_status
    }

    /// Returns the existing deterministic review finding count.
    pub fn review_finding_count(&self) -> usize {
        self.review_finding_count
    }

    /// Returns bounded warnings in rejected, first-order, second-order order.
    pub fn warnings(&self) -> &[KineticsVisualizationWarning] {
        &self.warnings
    }
}

fn check_fit_point_count(
    fit: KineticsFitResult,
    accepted_count: usize,
) -> Result<(), KineticsPlotDataError> {
    if fit.valid_point_count != accepted_count {
        return Err(KineticsPlotDataError::FitPointCountMismatch {
            model_kind: fit.model_kind,
            accepted_count,
            fit_count: fit.valid_point_count,
        });
    }

    Ok(())
}

fn check_fit_metadata(fit: KineticsFitResult) -> Result<(), KineticsPlotDataError> {
    if [fit.slope, fit.intercept, fit.r_squared]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(KineticsPlotDataError::NonFiniteFitMetadata {
            model_kind: fit.model_kind,
        });
    }

    Ok(())
}

fn accepted_time_domain(
    observations: &[KineticsPoint],
) -> Result<(f64, f64), KineticsPlotDataError> {
    let first = observations
        .first()
        .ok_or(KineticsPlotDataError::NoAcceptedPoints)?;
    if !first.time.is_finite() {
        return Err(KineticsPlotDataError::NonFiniteAcceptedTime);
    }

    let mut minimum = first.time;
    let mut maximum = first.time;
    for observation in &observations[1..] {
        if !observation.time.is_finite() {
            return Err(KineticsPlotDataError::NonFiniteAcceptedTime);
        }
        minimum = minimum.min(observation.time);
        maximum = maximum.max(observation.time);
    }

    let span = maximum - minimum;
    if !span.is_finite() {
        return Err(KineticsPlotDataError::NonFiniteAcceptedTimeSpan);
    }
    if span <= 0.0 {
        return Err(KineticsPlotDataError::NonPositiveAcceptedTimeSpan);
    }

    Ok((minimum, maximum))
}

fn build_model_data(
    fit: KineticsFitResult,
    minimum_time: f64,
    maximum_time: f64,
) -> (KineticsPlotModelData, Option<KineticsVisualizationWarning>) {
    let span = maximum_time - minimum_time;
    let mut candidates = Vec::with_capacity(CURVE_CANDIDATE_COUNT);

    for index in 0..CURVE_CANDIDATE_COUNT {
        let time = match index {
            0 => minimum_time,
            LAST_CURVE_CANDIDATE_INDEX => maximum_time,
            _ => minimum_time + span * index as f64 / LAST_CURVE_CANDIDATE_INDEX as f64,
        };
        let concentration = predict(fit, time);
        candidates.push(concentration.map(|concentration| KineticsPredictionPoint {
            time,
            concentration,
        }));
    }

    let retained_count = candidates.iter().flatten().count();
    let omitted_any = retained_count != CURVE_CANDIDATE_COUNT;
    let segments = renderable_segments(candidates);
    let warning = if retained_count < 2 {
        Some(omitted_warning(fit.model_kind))
    } else if omitted_any {
        Some(partial_warning(fit.model_kind))
    } else {
        None
    };

    (
        KineticsPlotModelData {
            model_kind: fit.model_kind,
            slope: fit.slope,
            intercept: fit.intercept,
            r_squared: fit.r_squared,
            segments,
        },
        warning,
    )
}

fn predict(fit: KineticsFitResult, time: f64) -> Option<f64> {
    match fit.model_kind {
        KineticsModelKind::FirstOrder => {
            let exponent = fit.intercept + fit.slope * time;
            if !exponent.is_finite() {
                return None;
            }
            let concentration = exponent.exp();
            concentration.is_finite().then_some(concentration)
        }
        KineticsModelKind::SecondOrder => {
            let denominator = fit.intercept + fit.slope * time;
            if !denominator.is_finite() || denominator <= 0.0 {
                return None;
            }
            let concentration = 1.0 / denominator;
            concentration.is_finite().then_some(concentration)
        }
    }
}

fn renderable_segments(
    candidates: Vec<Option<KineticsPredictionPoint>>,
) -> Vec<KineticsCurveSegment> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for candidate in candidates {
        match candidate {
            Some(point) => current.push(point),
            None => finish_segment(&mut current, &mut segments),
        }
    }
    finish_segment(&mut current, &mut segments);

    segments
}

fn finish_segment(
    current: &mut Vec<KineticsPredictionPoint>,
    segments: &mut Vec<KineticsCurveSegment>,
) {
    if current.len() >= 2 {
        segments.push(KineticsCurveSegment {
            points: std::mem::take(current),
        });
    } else {
        current.clear();
    }
}

fn partial_warning(model_kind: KineticsModelKind) -> KineticsVisualizationWarning {
    match model_kind {
        KineticsModelKind::FirstOrder => KineticsVisualizationWarning::FirstOrderPartiallyOmitted,
        KineticsModelKind::SecondOrder => KineticsVisualizationWarning::SecondOrderPartiallyOmitted,
    }
}

fn omitted_warning(model_kind: KineticsModelKind) -> KineticsVisualizationWarning {
    match model_kind {
        KineticsModelKind::FirstOrder => {
            KineticsVisualizationWarning::FirstOrderOmittedFewerThanTwoFinitePredictions
        }
        KineticsModelKind::SecondOrder => {
            KineticsVisualizationWarning::SecondOrderOmittedFewerThanTwoFinitePredictions
        }
    }
}

#[cfg(test)]
mod tests {
    use deepseek_science_common::{DataColumn, DataTable};

    use crate::kinetics::{
        KineticsAnalysisResult, KineticsColumns, KineticsComparisonBasis, KineticsFitResult,
        KineticsModelKind, KineticsPoint, KineticsReviewStatus, ValidatedKineticsInput,
    };

    use super::{
        accepted_time_domain, predict, renderable_segments, KineticsPlotData,
        KineticsPlotDataError, KineticsPlotModelData, KineticsPredictionPoint,
        KineticsVisualizationWarning, CURVE_CANDIDATE_COUNT,
    };

    fn pipeline(
        time_name: &str,
        concentration_name: &str,
        time: &[f64],
        concentration: &[f64],
    ) -> (
        KineticsColumns,
        ValidatedKineticsInput,
        KineticsAnalysisResult,
    ) {
        let table = DataTable::new(vec![
            DataColumn::numeric(time_name, time.to_vec()).expect("time column should be valid"),
            DataColumn::numeric(concentration_name, concentration.to_vec())
                .expect("concentration column should be valid"),
        ])
        .expect("table should be valid");
        let columns =
            KineticsColumns::new(time_name, concentration_name).expect("columns should be valid");
        let input =
            ValidatedKineticsInput::from_table(&table, &columns).expect("input should be valid");
        let analysis = KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");

        (columns, input, analysis)
    }

    fn standard_pipeline() -> (
        KineticsColumns,
        ValidatedKineticsInput,
        KineticsAnalysisResult,
    ) {
        pipeline(
            "Time_s",
            "Concentration_M",
            &[0.0, 1.0, 2.0, 3.0],
            &[2.0, 4.0 / 3.0, 1.0, 0.8],
        )
    }

    fn construct(
        columns: &KineticsColumns,
        input: &ValidatedKineticsInput,
        analysis: &KineticsAnalysisResult,
    ) -> KineticsPlotData {
        KineticsPlotData::from_analysis(input, columns, analysis)
            .expect("plot data should construct")
    }

    fn curve_points(model: &KineticsPlotModelData) -> Vec<KineticsPredictionPoint> {
        model
            .segments()
            .iter()
            .flat_map(|segment| segment.points().iter().copied())
            .collect()
    }

    fn assert_near(actual: f64, expected: f64) {
        let tolerance = 1.0e-12 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    #[test]
    fn plot_data_preserves_accepted_observations_and_caller_row_order() {
        let (columns, input, analysis) = pipeline(
            "time",
            "concentration",
            &[2.0, 99.0, 0.0, 1.0],
            &[1.0, 0.0, 2.0, 1.5],
        );

        let plot = construct(&columns, &input, &analysis);

        assert_eq!(plot.observations(), input.valid_points());
        assert_eq!(
            plot.observations()
                .iter()
                .map(|point| point.time)
                .collect::<Vec<_>>(),
            vec![2.0, 0.0, 1.0]
        );
    }

    #[test]
    fn sampled_curves_are_time_sorted_independently_of_observation_order() {
        let (columns, input, analysis) = pipeline(
            "time",
            "concentration",
            &[3.0, 1.0, 2.0],
            &[0.8, 4.0 / 3.0, 1.0],
        );
        let plot = construct(&columns, &input, &analysis);

        for model in [plot.first_order(), plot.second_order()] {
            let points = curve_points(model);
            assert!(points
                .windows(2)
                .all(|pair| pair[0].time() < pair[1].time()));
        }
    }

    #[test]
    fn exact_column_names_are_owned_and_case_sensitive() {
        let (columns, input, analysis) = standard_pipeline();
        let plot = construct(&columns, &input, &analysis);

        assert_eq!(plot.time_column(), "Time_s");
        assert_eq!(plot.concentration_column(), "Concentration_M");
    }

    #[test]
    fn counts_fit_metadata_and_analysis_summary_are_preserved() {
        let (columns, input, analysis) = pipeline(
            "time",
            "concentration",
            &[0.0, 99.0, 1.0, 2.0],
            &[1.0, 0.0, 0.8, 0.6],
        );
        let plot = construct(&columns, &input, &analysis);

        assert_eq!(plot.accepted_count(), input.valid_count());
        assert_eq!(plot.rejected_count(), input.rejected_count());
        assert_eq!(
            plot.first_order().model_kind(),
            KineticsModelKind::FirstOrder
        );
        assert_eq!(
            plot.first_order().slope(),
            analysis.comparison.first_order.slope
        );
        assert_eq!(
            plot.first_order().intercept(),
            analysis.comparison.first_order.intercept
        );
        assert_eq!(
            plot.first_order().r_squared(),
            analysis.comparison.first_order.r_squared
        );
        assert_eq!(
            plot.second_order().model_kind(),
            KineticsModelKind::SecondOrder
        );
        assert_eq!(
            plot.second_order().slope(),
            analysis.comparison.second_order.slope
        );
        assert_eq!(
            plot.second_order().intercept(),
            analysis.comparison.second_order.intercept
        );
        assert_eq!(
            plot.second_order().r_squared(),
            analysis.comparison.second_order.r_squared
        );
        assert_eq!(plot.preferred_model(), analysis.preferred_model());
        assert_eq!(plot.comparison_basis(), analysis.comparison_basis());
        assert_eq!(plot.review_status(), analysis.review_status());
        assert_eq!(plot.review_finding_count(), analysis.review.findings.len());
        assert_eq!(
            plot.comparison_basis(),
            KineticsComparisonBasis::FiniteRSquaredMvpHeuristic
        );
        assert_eq!(
            plot.review_status(),
            KineticsReviewStatus::PassedWithWarnings
        );
    }

    #[test]
    fn accepted_count_mismatch_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.valid_point_count += 1;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::AcceptedCountMismatch {
                validated_count: 4,
                analysis_count: 5,
            })
        );
    }

    #[test]
    fn rejected_count_mismatch_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.rejected_row_count = 1;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::RejectedCountMismatch {
                validated_count: 0,
                analysis_count: 1,
            })
        );
    }

    #[test]
    fn review_rejected_count_mismatch_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.review.rejected_row_count = 1;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::ReviewRejectedCountMismatch {
                validated_count: 0,
                review_count: 1,
            })
        );
    }

    #[test]
    fn first_order_fit_point_count_mismatch_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.comparison.first_order.valid_point_count = 3;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::FitPointCountMismatch {
                model_kind: KineticsModelKind::FirstOrder,
                accepted_count: 4,
                fit_count: 3,
            })
        );
    }

    #[test]
    fn second_order_fit_point_count_mismatch_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.comparison.second_order.valid_point_count = 3;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::FitPointCountMismatch {
                model_kind: KineticsModelKind::SecondOrder,
                accepted_count: 4,
                fit_count: 3,
            })
        );
    }

    #[test]
    fn empty_accepted_data_is_rejected_by_domain_construction() {
        assert_eq!(
            accepted_time_domain(&[]),
            Err(KineticsPlotDataError::NoAcceptedPoints)
        );
    }

    #[test]
    fn equal_accepted_times_are_rejected() {
        let points = [
            KineticsPoint {
                row_index: 0,
                time: 1.0,
                concentration: 2.0,
            },
            KineticsPoint {
                row_index: 1,
                time: 1.0,
                concentration: 1.0,
            },
        ];

        assert_eq!(
            accepted_time_domain(&points),
            Err(KineticsPlotDataError::NonPositiveAcceptedTimeSpan)
        );
    }

    #[test]
    fn non_finite_and_unrepresentable_time_ranges_are_rejected() {
        let non_finite = [KineticsPoint {
            row_index: 0,
            time: f64::NAN,
            concentration: 1.0,
        }];
        let unrepresentable = [
            KineticsPoint {
                row_index: 0,
                time: -f64::MAX,
                concentration: 1.0,
            },
            KineticsPoint {
                row_index: 1,
                time: f64::MAX,
                concentration: 1.0,
            },
        ];

        assert_eq!(
            accepted_time_domain(&non_finite),
            Err(KineticsPlotDataError::NonFiniteAcceptedTime)
        );
        assert_eq!(
            accepted_time_domain(&unrepresentable),
            Err(KineticsPlotDataError::NonFiniteAcceptedTimeSpan)
        );
    }

    #[test]
    fn non_finite_fit_metadata_is_rejected() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.comparison.first_order.r_squared = f64::NAN;

        assert_eq!(
            KineticsPlotData::from_analysis(&input, &columns, &analysis),
            Err(KineticsPlotDataError::NonFiniteFitMetadata {
                model_kind: KineticsModelKind::FirstOrder,
            })
        );
    }

    #[test]
    fn each_fully_valid_model_has_exactly_128_candidates_and_exact_endpoints() {
        let (columns, input, analysis) = standard_pipeline();
        let plot = construct(&columns, &input, &analysis);

        for model in [plot.first_order(), plot.second_order()] {
            let points = curve_points(model);
            assert_eq!(points.len(), CURVE_CANDIDATE_COUNT);
            assert_eq!(points.first().expect("first point").time(), 0.0);
            assert_eq!(points.last().expect("last point").time(), 3.0);
        }
    }

    #[test]
    fn first_order_predictions_use_existing_slope_and_intercept() {
        let (columns, input, analysis) = standard_pipeline();
        let plot = construct(&columns, &input, &analysis);

        for point in curve_points(plot.first_order()) {
            let expected =
                (plot.first_order().intercept() + plot.first_order().slope() * point.time()).exp();
            assert_near(point.concentration(), expected);
        }
    }

    #[test]
    fn second_order_predictions_use_existing_slope_and_intercept() {
        let (columns, input, analysis) = standard_pipeline();
        let plot = construct(&columns, &input, &analysis);

        for point in curve_points(plot.second_order()) {
            let expected = 1.0
                / (plot.second_order().intercept() + plot.second_order().slope() * point.time());
            assert_near(point.concentration(), expected);
        }
    }

    #[test]
    fn first_order_non_finite_prediction_is_omitted() {
        let fit = KineticsFitResult {
            model_kind: KineticsModelKind::FirstOrder,
            slope: 0.0,
            intercept: f64::MAX,
            rate_constant_k: 0.0,
            r_squared: 1.0,
            valid_point_count: 2,
        };

        assert_eq!(predict(fit, 0.0), None);
    }

    #[test]
    fn finite_first_order_underflowed_zero_is_retained() {
        let fit = KineticsFitResult {
            model_kind: KineticsModelKind::FirstOrder,
            slope: 0.0,
            intercept: -1_000.0,
            rate_constant_k: 0.0,
            r_squared: 1.0,
            valid_point_count: 2,
        };

        assert_eq!(predict(fit, 0.0), Some(0.0));
    }

    #[test]
    fn second_order_zero_and_negative_denominators_are_omitted() {
        let zero = KineticsFitResult {
            model_kind: KineticsModelKind::SecondOrder,
            slope: 0.0,
            intercept: 0.0,
            rate_constant_k: 0.0,
            r_squared: 1.0,
            valid_point_count: 2,
        };
        let negative = KineticsFitResult {
            intercept: -1.0,
            ..zero
        };

        assert_eq!(predict(zero, 0.0), None);
        assert_eq!(predict(negative, 0.0), None);
    }

    #[test]
    fn invalid_candidate_breaks_segments_without_crossing_the_gap() {
        let point = |time| {
            Some(KineticsPredictionPoint {
                time,
                concentration: 1.0,
            })
        };
        let segments =
            renderable_segments(vec![point(0.0), point(1.0), None, point(3.0), point(4.0)]);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].points()[0].time(), 0.0);
        assert_eq!(segments[0].points()[1].time(), 1.0);
        assert_eq!(segments[1].points()[0].time(), 3.0);
        assert_eq!(segments[1].points()[1].time(), 4.0);
    }

    #[test]
    fn segments_with_fewer_than_two_points_are_filtered_during_construction() {
        let point = |time| {
            Some(KineticsPredictionPoint {
                time,
                concentration: 1.0,
            })
        };
        let segments = renderable_segments(vec![point(0.0), None, point(2.0)]);

        assert!(segments.is_empty());
    }

    #[test]
    fn first_order_partial_and_omitted_warnings_are_distinct() {
        let (columns, input, mut partial_analysis) =
            pipeline("time", "concentration", &[0.0, 1.0, 2.0], &[1.0, 0.8, 0.6]);
        partial_analysis.comparison.first_order.intercept = 700.0;
        partial_analysis.comparison.first_order.slope = 10.0;
        let partial = construct(&columns, &input, &partial_analysis);

        let mut omitted_analysis = partial_analysis.clone();
        omitted_analysis.comparison.first_order.intercept = f64::MAX;
        omitted_analysis.comparison.first_order.slope = 0.0;
        let omitted = construct(&columns, &input, &omitted_analysis);

        assert!(partial
            .warnings()
            .contains(&KineticsVisualizationWarning::FirstOrderPartiallyOmitted));
        assert!(!partial.first_order().segments().is_empty());
        assert!(omitted.warnings().contains(
            &KineticsVisualizationWarning::FirstOrderOmittedFewerThanTwoFinitePredictions
        ));
        assert!(omitted.first_order().segments().is_empty());
    }

    #[test]
    fn second_order_partial_curve_starts_after_invalid_candidates() {
        let (columns, input, mut analysis) =
            pipeline("time", "concentration", &[0.0, 0.5, 1.0], &[2.0, 1.0, 0.5]);
        analysis.comparison.second_order.intercept = -0.5;
        analysis.comparison.second_order.slope = 1.0;

        let plot = construct(&columns, &input, &analysis);

        assert!(plot
            .warnings()
            .contains(&KineticsVisualizationWarning::SecondOrderPartiallyOmitted));
        let first_retained = plot.second_order().segments()[0].points()[0];
        assert!(first_retained.time() > 0.5);
    }

    #[test]
    fn second_order_with_one_finite_prediction_is_omitted() {
        let (columns, input, mut analysis) =
            pipeline("time", "concentration", &[0.0, 0.5, 1.0], &[2.0, 1.0, 0.5]);
        analysis.comparison.second_order.intercept = -0.9999;
        analysis.comparison.second_order.slope = 1.0;

        let plot = construct(&columns, &input, &analysis);

        assert!(plot.warnings().contains(
            &KineticsVisualizationWarning::SecondOrderOmittedFewerThanTwoFinitePredictions
        ));
        assert!(plot.second_order().segments().is_empty());
    }

    #[test]
    fn omitted_preferred_curve_does_not_change_preference_or_review() {
        let (columns, input, mut analysis) = pipeline(
            "time",
            "concentration",
            &[0.0, 1.0, 2.0],
            &[1.0, (-0.25_f64).exp(), (-0.5_f64).exp()],
        );
        assert_eq!(analysis.preferred_model(), KineticsModelKind::FirstOrder);
        let expected_review = analysis.review_status();
        analysis.comparison.first_order.intercept = f64::MAX;
        analysis.comparison.first_order.slope = 0.0;

        let plot = construct(&columns, &input, &analysis);

        assert_eq!(plot.preferred_model(), KineticsModelKind::FirstOrder);
        assert_eq!(plot.review_status(), expected_review);
        assert!(plot.warnings().contains(
            &KineticsVisualizationWarning::FirstOrderOmittedFewerThanTwoFinitePredictions
        ));
    }

    #[test]
    fn rejected_warning_has_no_rejected_row_payload() {
        let (columns, input, analysis) = pipeline(
            "time",
            "concentration",
            &[0.0, 999.0, 1.0],
            &[1.0, 0.0, 0.5],
        );
        let plot = construct(&columns, &input, &analysis);

        assert_eq!(
            plot.warnings().first(),
            Some(&KineticsVisualizationWarning::RejectedRowsPresent)
        );
        assert!(plot
            .observations()
            .iter()
            .all(|point| point.row_index != 1 && point.time != 999.0));
        assert!(!format!("{:?}", plot.warnings()).contains("999"));
    }

    #[test]
    fn warning_order_is_fixed_and_count_is_bounded_to_three() {
        let (columns, input, mut analysis) = pipeline(
            "time",
            "concentration",
            &[0.0, 99.0, 1.0, 2.0],
            &[1.0, 0.0, 0.8, 0.6],
        );
        analysis.comparison.first_order.intercept = 700.0;
        analysis.comparison.first_order.slope = 10.0;
        analysis.comparison.second_order.intercept = -1.0;
        analysis.comparison.second_order.slope = 0.0;

        let plot = construct(&columns, &input, &analysis);

        assert_eq!(
            plot.warnings(),
            &[
                KineticsVisualizationWarning::RejectedRowsPresent,
                KineticsVisualizationWarning::FirstOrderPartiallyOmitted,
                KineticsVisualizationWarning::SecondOrderOmittedFewerThanTwoFinitePredictions,
            ]
        );
        assert!(plot.warnings().len() <= 3);
    }

    #[test]
    fn construction_uses_supplied_fit_without_refitting() {
        let (columns, input, mut analysis) = standard_pipeline();
        analysis.comparison.first_order.slope = 0.0;
        analysis.comparison.first_order.intercept = 2.0_f64.ln();

        let plot = construct(&columns, &input, &analysis);
        let points = curve_points(plot.first_order());

        assert_eq!(plot.first_order().slope(), 0.0);
        assert_eq!(plot.first_order().intercept(), 2.0_f64.ln());
        assert!(points
            .iter()
            .all(|point| (point.concentration() - 2.0).abs() <= 1.0e-12));
    }
}
