//! Deterministic, pure in-memory SVG rendering for kinetics plot data.

use thiserror::Error;

use crate::kinetics::{
    KineticsComparisonBasis, KineticsModelKind, KineticsPoint, KineticsReviewStatus,
};
use crate::kinetics_plot::{
    KineticsCurveSegment, KineticsPlotData, KineticsPlotModelData, KineticsVisualizationWarning,
};

const SVG_MAX_BYTES: usize = 4 * 1024 * 1024;
const PLOT_LEFT: f64 = 96.0;
const PLOT_TOP: f64 = 72.0;
const PLOT_WIDTH: f64 = 624.0;
const PLOT_HEIGHT: f64 = 360.0;
const PLOT_BOTTOM: f64 = PLOT_TOP + PLOT_HEIGHT;
const TICK_COUNT: usize = 6;

const ROOT_LINE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 960 640\" width=\"960\" height=\"640\" role=\"img\" aria-labelledby=\"plot-title plot-desc\" font-family=\"system-ui, sans-serif\">\n";

/// Errors raised while rendering deterministic kinetics SVG text.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum KineticsSvgRenderError {
    /// A column label contains a character forbidden by the SVG text contract.
    #[error("kinetics SVG label contains forbidden character U+{code_point:04X}")]
    InvalidLabelCharacter {
        /// Rejected Unicode scalar value.
        code_point: u32,
    },

    /// Accepted observation times cannot produce finite ordered axis bounds.
    #[error("kinetics SVG accepted time range is not renderable")]
    NonRenderableXRange,

    /// Accepted observation concentrations cannot produce finite ordered bounds.
    #[error("kinetics SVG accepted concentration range is not renderable")]
    NonRenderableAcceptedYRange,

    /// A deterministic tick position is not finite.
    #[error("kinetics SVG tick position is not finite")]
    NonFiniteTick,

    /// An accepted observation cannot be mapped into the fixed plot panel.
    #[error("kinetics SVG accepted observation coordinate is not finite")]
    NonFiniteObservationCoordinate,

    /// A value supplied to the deterministic numeric formatter is not finite.
    #[error("kinetics SVG numeric value is not finite")]
    NonFiniteNumber,

    /// The completed SVG would exceed its fixed byte limit.
    #[error("kinetics SVG exceeds the {maximum}-byte output limit")]
    SvgSizeExceeded {
        /// Maximum permitted UTF-8 byte length.
        maximum: usize,
    },

    /// Immutable plot data violated a renderer assumption guaranteed by Phase 5.1.
    #[error("kinetics SVG plot data violates an internal contract")]
    InternalContractViolation,
}

/// Renders one fixed, standalone kinetics SVG document entirely in memory.
///
/// The renderer consumes accepted observations and already-sampled curve
/// segments from [`KineticsPlotData`]. It does not parse input, validate rows,
/// fit models, regenerate predictions, access paths, or perform file IO.
pub fn render_kinetics_svg(data: &KineticsPlotData) -> Result<String, KineticsSvgRenderError> {
    render_with_limit(data, SVG_MAX_BYTES)
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: f64,
    max: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelWarning {
    PartiallyOmitted,
    FewerThanTwoFinitePredictions,
    RenderRangeNotRepresentable,
}

#[derive(Clone, Copy, Default)]
struct RenderWarnings {
    rejected_rows: bool,
    first_order: Option<ModelWarning>,
    second_order: Option<ModelWarning>,
}

impl RenderWarnings {
    fn from_plot_data(data: &KineticsPlotData) -> Result<Self, KineticsSvgRenderError> {
        let mut warnings = Self::default();
        for warning in data.warnings() {
            match warning {
                KineticsVisualizationWarning::RejectedRowsPresent => {
                    warnings.rejected_rows = true;
                }
                KineticsVisualizationWarning::FirstOrderPartiallyOmitted => {
                    warnings.first_order = Some(ModelWarning::PartiallyOmitted);
                }
                KineticsVisualizationWarning::FirstOrderOmittedFewerThanTwoFinitePredictions => {
                    warnings.first_order = Some(ModelWarning::FewerThanTwoFinitePredictions);
                }
                KineticsVisualizationWarning::SecondOrderPartiallyOmitted => {
                    warnings.second_order = Some(ModelWarning::PartiallyOmitted);
                }
                KineticsVisualizationWarning::SecondOrderOmittedFewerThanTwoFinitePredictions => {
                    warnings.second_order = Some(ModelWarning::FewerThanTwoFinitePredictions);
                }
            }
        }

        if warnings.rejected_rows != (data.rejected_count() > 0) {
            return Err(KineticsSvgRenderError::InternalContractViolation);
        }
        Ok(warnings)
    }

    fn count(self) -> usize {
        usize::from(self.rejected_rows)
            + usize::from(self.first_order.is_some())
            + usize::from(self.second_order.is_some())
    }

    fn set_model_range_warning(&mut self, model_kind: KineticsModelKind) {
        match model_kind {
            KineticsModelKind::FirstOrder => {
                self.first_order = Some(ModelWarning::RenderRangeNotRepresentable);
            }
            KineticsModelKind::SecondOrder => {
                self.second_order = Some(ModelWarning::RenderRangeNotRepresentable);
            }
        }
    }
}

struct SvgBuffer {
    text: String,
    maximum: usize,
}

impl SvgBuffer {
    fn new(maximum: usize) -> Self {
        Self {
            text: String::new(),
            maximum,
        }
    }

    fn push(&mut self, value: &str) -> Result<(), KineticsSvgRenderError> {
        let next_len = self.text.len().checked_add(value.len()).ok_or(
            KineticsSvgRenderError::SvgSizeExceeded {
                maximum: self.maximum,
            },
        )?;
        if next_len > self.maximum {
            return Err(KineticsSvgRenderError::SvgSizeExceeded {
                maximum: self.maximum,
            });
        }
        self.text.push_str(value);
        Ok(())
    }

    fn finish(self) -> Result<String, KineticsSvgRenderError> {
        if self.text.len() > self.maximum {
            return Err(KineticsSvgRenderError::SvgSizeExceeded {
                maximum: self.maximum,
            });
        }
        Ok(self.text)
    }
}

struct RenderDecision<'a> {
    first_order: Option<&'a [KineticsCurveSegment]>,
    second_order: Option<&'a [KineticsCurveSegment]>,
    y_bounds: Bounds,
    warnings: RenderWarnings,
}

fn render_with_limit(
    data: &KineticsPlotData,
    maximum: usize,
) -> Result<String, KineticsSvgRenderError> {
    validate_plot_data_contract(data)?;
    let time_label = safe_label(data.time_column())?;
    let concentration_label = safe_label(data.concentration_column())?;
    let x_bounds = observation_x_bounds(data.observations())?;
    let decision = render_decision(data, x_bounds)?;
    let x_ticks = ticks(x_bounds)?;
    let y_ticks = ticks(decision.y_bounds)?;

    let first_r_squared = format_finite(data.first_order().r_squared())?;
    let second_r_squared = format_finite(data.second_order().r_squared())?;
    let preference = preference_label(data.preferred_model());
    let review_status = review_status_label(data.review_status());
    let warning_count = decision.warnings.count();

    let mut svg = SvgBuffer::new(maximum);
    svg.push(ROOT_LINE)?;
    svg.push("<title id=\"plot-title\">Kinetics concentration versus time</title>\n")?;

    let desc = format!(
        "<desc id=\"plot-desc\">Time column: {time_label}; concentration column: {concentration_label}; accepted observations: {}; rejected rows: {}; first-order r_squared: {first_r_squared}; second-order r_squared: {second_r_squared}; MVP heuristic preference: {preference}; deterministic review status: {review_status}; visualization warnings: {warning_count}</desc>\n",
        data.accepted_count(),
        data.rejected_count(),
    );
    svg.push(&desc)?;
    svg.push("<rect x=\"0\" y=\"0\" width=\"960\" height=\"640\" fill=\"#ffffff\"/>\n")?;

    render_plot_background_and_grid(&mut svg, &x_ticks, &y_ticks, x_bounds, decision.y_bounds)?;
    render_axes_and_ticks(&mut svg, &x_ticks, &y_ticks, x_bounds, decision.y_bounds)?;
    render_model_segments(
        &mut svg,
        KineticsModelKind::FirstOrder,
        decision.first_order,
        x_bounds,
        decision.y_bounds,
    )?;
    render_model_segments(
        &mut svg,
        KineticsModelKind::SecondOrder,
        decision.second_order,
        x_bounds,
        decision.y_bounds,
    )?;
    render_observations(&mut svg, data.observations(), x_bounds, decision.y_bounds)?;
    render_axis_labels(&mut svg, &time_label, &concentration_label)?;
    render_legend(&mut svg)?;
    render_summary(
        &mut svg,
        data,
        &first_r_squared,
        &second_r_squared,
        preference,
        review_status,
        warning_count,
    )?;
    render_warnings(&mut svg, decision.warnings)?;
    svg.push("</svg>\n")?;
    svg.finish()
}

fn validate_plot_data_contract(data: &KineticsPlotData) -> Result<(), KineticsSvgRenderError> {
    if data.accepted_count() != data.observations().len()
        || data.first_order().model_kind() != KineticsModelKind::FirstOrder
        || data.second_order().model_kind() != KineticsModelKind::SecondOrder
        || data.comparison_basis() != KineticsComparisonBasis::FiniteRSquaredMvpHeuristic
    {
        return Err(KineticsSvgRenderError::InternalContractViolation);
    }
    Ok(())
}

fn render_decision(
    data: &KineticsPlotData,
    x_bounds: Bounds,
) -> Result<RenderDecision<'_>, KineticsSvgRenderError> {
    let mut warnings = RenderWarnings::from_plot_data(data)?;
    let mut included_values = observation_concentrations(data.observations())?;
    y_bounds(&included_values).map_err(|_| KineticsSvgRenderError::NonRenderableAcceptedYRange)?;

    let first_order = consider_model(
        data.first_order(),
        &mut included_values,
        x_bounds,
        &mut warnings,
    );
    let second_order = consider_model(
        data.second_order(),
        &mut included_values,
        x_bounds,
        &mut warnings,
    );
    let final_y_bounds = y_bounds(&included_values)
        .map_err(|_| KineticsSvgRenderError::NonRenderableAcceptedYRange)?;

    Ok(RenderDecision {
        first_order,
        second_order,
        y_bounds: final_y_bounds,
        warnings,
    })
}

fn consider_model<'a>(
    model: &'a KineticsPlotModelData,
    included_values: &mut Vec<f64>,
    x_bounds: Bounds,
    warnings: &mut RenderWarnings,
) -> Option<&'a [KineticsCurveSegment]> {
    if model.segments().is_empty() {
        return None;
    }

    let original_len = included_values.len();
    for segment in model.segments() {
        for point in segment.points() {
            if !point.time().is_finite() || !point.concentration().is_finite() {
                included_values.truncate(original_len);
                warnings.set_model_range_warning(model.model_kind());
                return None;
            }
            included_values.push(point.concentration());
        }
    }

    let Ok(candidate_bounds) = y_bounds(included_values) else {
        included_values.truncate(original_len);
        warnings.set_model_range_warning(model.model_kind());
        return None;
    };
    if model_coordinates_are_finite(model.segments(), x_bounds, candidate_bounds) {
        Some(model.segments())
    } else {
        included_values.truncate(original_len);
        warnings.set_model_range_warning(model.model_kind());
        None
    }
}

fn observation_concentrations(
    observations: &[KineticsPoint],
) -> Result<Vec<f64>, KineticsSvgRenderError> {
    if observations.is_empty() {
        return Err(KineticsSvgRenderError::NonRenderableAcceptedYRange);
    }
    let mut values = Vec::with_capacity(observations.len());
    for observation in observations {
        if !observation.concentration.is_finite() {
            return Err(KineticsSvgRenderError::NonRenderableAcceptedYRange);
        }
        values.push(observation.concentration);
    }
    Ok(values)
}

fn observation_x_bounds(observations: &[KineticsPoint]) -> Result<Bounds, KineticsSvgRenderError> {
    let first = observations
        .first()
        .ok_or(KineticsSvgRenderError::NonRenderableXRange)?;
    if !first.time.is_finite() {
        return Err(KineticsSvgRenderError::NonRenderableXRange);
    }
    let mut minimum = first.time;
    let mut maximum = first.time;
    for observation in &observations[1..] {
        if !observation.time.is_finite() {
            return Err(KineticsSvgRenderError::NonRenderableXRange);
        }
        minimum = minimum.min(observation.time);
        maximum = maximum.max(observation.time);
    }

    let span = maximum - minimum;
    if !span.is_finite() || span <= 0.0 {
        return Err(KineticsSvgRenderError::NonRenderableXRange);
    }
    let padding = span * 0.05;
    let bounds = Bounds {
        min: minimum - padding,
        max: maximum + padding,
    };
    validate_bounds(bounds).map_err(|_| KineticsSvgRenderError::NonRenderableXRange)?;
    Ok(bounds)
}

fn y_bounds(values: &[f64]) -> Result<Bounds, KineticsSvgRenderError> {
    let first = *values
        .first()
        .ok_or(KineticsSvgRenderError::NonRenderableAcceptedYRange)?;
    if !first.is_finite() {
        return Err(KineticsSvgRenderError::NonRenderableAcceptedYRange);
    }
    let mut minimum = first;
    let mut maximum = first;
    for value in &values[1..] {
        if !value.is_finite() {
            return Err(KineticsSvgRenderError::NonRenderableAcceptedYRange);
        }
        minimum = minimum.min(*value);
        maximum = maximum.max(*value);
    }

    let bounds = if minimum >= 0.0 {
        let upper = if maximum == 0.0 {
            1e-12
        } else {
            maximum + maximum * 0.05
        };
        Bounds {
            min: 0.0,
            max: upper,
        }
    } else {
        let span = maximum - minimum;
        if !span.is_finite() {
            return Err(KineticsSvgRenderError::NonRenderableAcceptedYRange);
        }
        let padding = if span == 0.0 {
            (minimum.abs() * 0.05).max(1e-12)
        } else {
            span * 0.05
        };
        Bounds {
            min: minimum - padding,
            max: maximum + padding,
        }
    };

    validate_bounds(bounds).map_err(|_| KineticsSvgRenderError::NonRenderableAcceptedYRange)?;
    Ok(bounds)
}

fn validate_bounds(bounds: Bounds) -> Result<(), KineticsSvgRenderError> {
    let span = bounds.max - bounds.min;
    if !bounds.min.is_finite() || !bounds.max.is_finite() || !span.is_finite() || span <= 0.0 {
        return Err(KineticsSvgRenderError::NonFiniteNumber);
    }
    Ok(())
}

fn ticks(bounds: Bounds) -> Result<[f64; TICK_COUNT], KineticsSvgRenderError> {
    validate_bounds(bounds).map_err(|_| KineticsSvgRenderError::NonFiniteTick)?;
    let span = bounds.max - bounds.min;
    let mut ticks = [0.0; TICK_COUNT];
    for (index, tick) in ticks.iter_mut().enumerate() {
        *tick = match index {
            0 => bounds.min,
            index if index == TICK_COUNT - 1 => bounds.max,
            _ => bounds.min + (span / (TICK_COUNT - 1) as f64) * index as f64,
        };
        if !tick.is_finite() {
            return Err(KineticsSvgRenderError::NonFiniteTick);
        }
    }
    Ok(ticks)
}

fn map_x(value: f64, bounds: Bounds) -> Result<f64, KineticsSvgRenderError> {
    let coordinate = PLOT_LEFT + (value - bounds.min) / (bounds.max - bounds.min) * PLOT_WIDTH;
    if coordinate.is_finite() {
        Ok(coordinate)
    } else {
        Err(KineticsSvgRenderError::NonFiniteObservationCoordinate)
    }
}

fn map_y(value: f64, bounds: Bounds) -> Result<f64, KineticsSvgRenderError> {
    let coordinate = PLOT_BOTTOM - (value - bounds.min) / (bounds.max - bounds.min) * PLOT_HEIGHT;
    if coordinate.is_finite() {
        Ok(coordinate)
    } else {
        Err(KineticsSvgRenderError::NonFiniteObservationCoordinate)
    }
}

fn model_coordinates_are_finite(
    segments: &[KineticsCurveSegment],
    x_bounds: Bounds,
    y_bounds: Bounds,
) -> bool {
    segments.iter().all(|segment| {
        segment.points().iter().all(|point| {
            map_x(point.time(), x_bounds).is_ok() && map_y(point.concentration(), y_bounds).is_ok()
        })
    })
}

fn render_plot_background_and_grid(
    svg: &mut SvgBuffer,
    x_ticks: &[f64; TICK_COUNT],
    y_ticks: &[f64; TICK_COUNT],
    x_bounds: Bounds,
    y_bounds: Bounds,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"plot-grid\">\n")?;
    svg.push("<rect x=\"96\" y=\"72\" width=\"624\" height=\"360\" fill=\"#ffffff\"/>\n")?;
    for tick in x_ticks {
        let x = format_finite(map_x(*tick, x_bounds)?)?;
        svg.push(&format!(
            "<line x1=\"{x}\" y1=\"72\" x2=\"{x}\" y2=\"432\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>\n"
        ))?;
    }
    for tick in y_ticks {
        let y = format_finite(map_y(*tick, y_bounds)?)?;
        svg.push(&format!(
            "<line x1=\"96\" y1=\"{y}\" x2=\"720\" y2=\"{y}\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>\n"
        ))?;
    }
    svg.push("</g>\n")
}

fn render_axes_and_ticks(
    svg: &mut SvgBuffer,
    x_ticks: &[f64; TICK_COUNT],
    y_ticks: &[f64; TICK_COUNT],
    x_bounds: Bounds,
    y_bounds: Bounds,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"axes\" fill=\"#334155\">\n")?;
    svg.push("<line x1=\"96\" y1=\"432\" x2=\"720\" y2=\"432\" stroke=\"#334155\" stroke-width=\"1.5\"/>\n")?;
    svg.push("<line x1=\"96\" y1=\"72\" x2=\"96\" y2=\"432\" stroke=\"#334155\" stroke-width=\"1.5\"/>\n")?;
    for tick in x_ticks {
        let x = format_finite(map_x(*tick, x_bounds)?)?;
        let label = format_finite(*tick)?;
        svg.push(&format!(
            "<line x1=\"{x}\" y1=\"432\" x2=\"{x}\" y2=\"438\" stroke=\"#334155\" stroke-width=\"1\"/>\n"
        ))?;
        svg.push(&format!(
            "<text x=\"{x}\" y=\"454\" text-anchor=\"middle\" font-size=\"11\">{label}</text>\n"
        ))?;
    }
    for tick in y_ticks {
        let y = format_finite(map_y(*tick, y_bounds)?)?;
        let label = format_finite(*tick)?;
        svg.push(&format!(
            "<line x1=\"90\" y1=\"{y}\" x2=\"96\" y2=\"{y}\" stroke=\"#334155\" stroke-width=\"1\"/>\n"
        ))?;
        svg.push(&format!(
            "<text x=\"84\" y=\"{y}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-size=\"11\">{label}</text>\n"
        ))?;
    }
    svg.push("</g>\n")
}

fn render_model_segments(
    svg: &mut SvgBuffer,
    model_kind: KineticsModelKind,
    segments: Option<&[KineticsCurveSegment]>,
    x_bounds: Bounds,
    y_bounds: Bounds,
) -> Result<(), KineticsSvgRenderError> {
    let (id, stroke, dash) = match model_kind {
        KineticsModelKind::FirstOrder => ("first-order-curves", "#005ea8", None),
        KineticsModelKind::SecondOrder => ("second-order-curves", "#a23b00", Some("8 5")),
    };
    svg.push(&format!("<g id=\"{id}\">\n"))?;
    if let Some(segments) = segments {
        for segment in segments {
            svg.push("<polyline points=\"")?;
            for (index, point) in segment.points().iter().enumerate() {
                if index > 0 {
                    svg.push(" ")?;
                }
                let x = map_x(point.time(), x_bounds)
                    .map_err(|_| KineticsSvgRenderError::InternalContractViolation)?;
                let y = map_y(point.concentration(), y_bounds)
                    .map_err(|_| KineticsSvgRenderError::InternalContractViolation)?;
                svg.push(&format!("{},{}", format_finite(x)?, format_finite(y)?))?;
            }
            if let Some(dash) = dash {
                svg.push(&format!(
                    "\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2.5\" stroke-dasharray=\"{dash}\"/>\n"
                ))?;
            } else {
                svg.push(&format!(
                    "\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2.5\"/>\n"
                ))?;
            }
        }
    }
    svg.push("</g>\n")
}

fn render_observations(
    svg: &mut SvgBuffer,
    observations: &[KineticsPoint],
    x_bounds: Bounds,
    y_bounds: Bounds,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"observations\">\n")?;
    for observation in observations {
        let x = format_finite(map_x(observation.time, x_bounds)?)?;
        let y = format_finite(map_y(observation.concentration, y_bounds)?)?;
        svg.push(&format!(
            "<circle cx=\"{x}\" cy=\"{y}\" r=\"3.5\" fill=\"#111827\"/>\n"
        ))?;
    }
    svg.push("</g>\n")
}

fn render_axis_labels(
    svg: &mut SvgBuffer,
    time_label: &str,
    concentration_label: &str,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"axis-labels\" fill=\"#111827\">\n")?;
    svg.push(&format!(
        "<text x=\"408\" y=\"476\" text-anchor=\"middle\" font-size=\"14\">{time_label}</text>\n"
    ))?;
    svg.push(&format!(
        "<text x=\"32\" y=\"252\" text-anchor=\"middle\" font-size=\"14\" transform=\"rotate(-90 32 252)\">{concentration_label}</text>\n"
    ))?;
    svg.push("</g>\n")
}

fn render_legend(svg: &mut SvgBuffer) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"legend\" fill=\"#111827\">\n")?;
    svg.push("<rect x=\"752\" y=\"72\" width=\"176\" height=\"104\" fill=\"#ffffff\" stroke=\"#334155\" stroke-width=\"1\"/>\n")?;
    svg.push("<circle cx=\"764\" cy=\"96\" r=\"3.5\" fill=\"#111827\"/>\n")?;
    svg.push("<text x=\"780\" y=\"100\" font-size=\"12\">observed data</text>\n")?;
    svg.push("<line x1=\"756\" y1=\"124\" x2=\"780\" y2=\"124\" stroke=\"#005ea8\" stroke-width=\"2.5\"/>\n")?;
    svg.push("<text x=\"790\" y=\"128\" font-size=\"12\">first-order fit</text>\n")?;
    svg.push("<line x1=\"756\" y1=\"152\" x2=\"780\" y2=\"152\" stroke=\"#a23b00\" stroke-width=\"2.5\" stroke-dasharray=\"8 5\"/>\n")?;
    svg.push("<text x=\"790\" y=\"156\" font-size=\"12\">second-order fit</text>\n")?;
    svg.push("</g>\n")
}

fn render_summary(
    svg: &mut SvgBuffer,
    data: &KineticsPlotData,
    first_r_squared: &str,
    second_r_squared: &str,
    preference: &str,
    review_status: &str,
    warning_count: usize,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"fit-summary\" fill=\"#111827\">\n")?;
    for (y, text) in [
        (210, format!("first-order r_squared: {first_r_squared}")),
        (234, format!("second-order r_squared: {second_r_squared}")),
        (258, format!("MVP heuristic preference: {preference}")),
        (
            282,
            format!(
                "comparison basis: {}",
                comparison_basis_label(data.comparison_basis())
            ),
        ),
        (306, format!("deterministic review status: {review_status}")),
        (
            330,
            format!(
                "deterministic review findings: {}",
                data.review_finding_count()
            ),
        ),
        (
            354,
            format!("accepted observations: {}", data.accepted_count()),
        ),
        (378, format!("rejected rows: {}", data.rejected_count())),
        (402, format!("visualization warnings: {warning_count}")),
    ] {
        svg.push(&format!(
            "<text x=\"752\" y=\"{y}\" font-size=\"10\">{text}</text>\n"
        ))?;
    }
    svg.push("</g>\n")
}

fn render_warnings(
    svg: &mut SvgBuffer,
    warnings: RenderWarnings,
) -> Result<(), KineticsSvgRenderError> {
    svg.push("<g id=\"visualization-warnings\" fill=\"#111827\">\n")?;
    let mut texts = Vec::with_capacity(3);
    if warnings.rejected_rows {
        texts.push("rejected rows not displayed");
    }
    if let Some(warning) = warnings.first_order {
        texts.push(model_warning_text(KineticsModelKind::FirstOrder, warning));
    }
    if let Some(warning) = warnings.second_order {
        texts.push(model_warning_text(KineticsModelKind::SecondOrder, warning));
    }
    for (text, baseline) in texts.into_iter().zip([536, 568, 600]) {
        svg.push(&format!(
            "<text x=\"96\" y=\"{baseline}\" font-size=\"12\">{text}</text>\n"
        ))?;
    }
    svg.push("</g>\n")
}

fn model_warning_text(model: KineticsModelKind, warning: ModelWarning) -> &'static str {
    match (model, warning) {
        (KineticsModelKind::FirstOrder, ModelWarning::PartiallyOmitted) => {
            "first-order fit partially omitted"
        }
        (KineticsModelKind::FirstOrder, ModelWarning::FewerThanTwoFinitePredictions) => {
            "first-order fit omitted: fewer than two finite predictions"
        }
        (KineticsModelKind::FirstOrder, ModelWarning::RenderRangeNotRepresentable) => {
            "first-order fit omitted: render range not representable"
        }
        (KineticsModelKind::SecondOrder, ModelWarning::PartiallyOmitted) => {
            "second-order fit partially omitted"
        }
        (KineticsModelKind::SecondOrder, ModelWarning::FewerThanTwoFinitePredictions) => {
            "second-order fit omitted: fewer than two finite predictions"
        }
        (KineticsModelKind::SecondOrder, ModelWarning::RenderRangeNotRepresentable) => {
            "second-order fit omitted: render range not representable"
        }
    }
}

fn preference_label(model: KineticsModelKind) -> &'static str {
    match model {
        KineticsModelKind::FirstOrder => "first-order",
        KineticsModelKind::SecondOrder => "second-order",
    }
}

fn comparison_basis_label(basis: KineticsComparisonBasis) -> &'static str {
    match basis {
        KineticsComparisonBasis::FiniteRSquaredMvpHeuristic => "finite r_squared MVP heuristic",
    }
}

fn review_status_label(status: KineticsReviewStatus) -> &'static str {
    match status {
        KineticsReviewStatus::Passed => "passed",
        KineticsReviewStatus::PassedWithWarnings => "passed with warnings",
        KineticsReviewStatus::Failed => "failed",
    }
}

fn safe_label(value: &str) -> Result<String, KineticsSvgRenderError> {
    for character in value.chars() {
        if !is_xml_1_0_character(character)
            || character.is_control()
            || matches!(character, '\u{2028}' | '\u{2029}')
        {
            return Err(KineticsSvgRenderError::InvalidLabelCharacter {
                code_point: character as u32,
            });
        }
    }

    let count = value.chars().count();
    let displayed = if count > 48 {
        let mut displayed: String = value.chars().take(47).collect();
        displayed.push('\u{2026}');
        displayed
    } else {
        value.to_owned()
    };

    let mut escaped = String::with_capacity(displayed.len());
    for character in displayed.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    Ok(escaped)
}

fn is_xml_1_0_character(character: char) -> bool {
    matches!(
        character as u32,
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}

fn format_finite(value: f64) -> Result<String, KineticsSvgRenderError> {
    if !value.is_finite() {
        return Err(KineticsSvgRenderError::NonFiniteNumber);
    }
    if value == 0.0 {
        return Ok("0".to_string());
    }

    let absolute = value.abs();
    let formatted = if (1e-4..1e6).contains(&absolute) {
        trim_fraction(format!("{value:.6}"))
    } else {
        let raw = format!("{value:.6e}");
        let (mantissa, exponent) = raw
            .split_once('e')
            .ok_or(KineticsSvgRenderError::NonFiniteNumber)?;
        let exponent = exponent
            .parse::<i32>()
            .map_err(|_| KineticsSvgRenderError::NonFiniteNumber)?;
        format!("{}e{exponent:+}", trim_fraction(mantissa.to_string()))
    };

    if formatted == "-0" || formatted.starts_with("-0e") {
        Ok(formatted.trim_start_matches('-').to_string())
    } else {
        Ok(formatted)
    }
}

fn trim_fraction(mut value: String) -> String {
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use deepseek_science_common::{DataColumn, DataTable};

    use crate::{
        KineticsAnalysisResult, KineticsColumns, KineticsModelKind, KineticsPlotData,
        KineticsReviewStatus, ValidatedKineticsInput,
    };

    use super::{
        format_finite, map_x, map_y, observation_x_bounds, render_kinetics_svg, render_with_limit,
        review_status_label, safe_label, ticks, y_bounds, Bounds, KineticsSvgRenderError,
        PLOT_BOTTOM, ROOT_LINE,
    };

    fn plot_data(
        time_name: &str,
        concentration_name: &str,
        times: Vec<f64>,
        concentrations: Vec<f64>,
    ) -> KineticsPlotData {
        plot_data_with_analysis(time_name, concentration_name, times, concentrations, |_| {})
    }

    fn plot_data_with_analysis(
        time_name: &str,
        concentration_name: &str,
        times: Vec<f64>,
        concentrations: Vec<f64>,
        update: impl FnOnce(&mut KineticsAnalysisResult),
    ) -> KineticsPlotData {
        let table = DataTable::new(vec![
            DataColumn::numeric(time_name, times).expect("time should be valid"),
            DataColumn::numeric(concentration_name, concentrations)
                .expect("concentration should be valid"),
        ])
        .expect("table should be valid");
        let columns =
            KineticsColumns::new(time_name, concentration_name).expect("columns should be valid");
        let input =
            ValidatedKineticsInput::from_table(&table, &columns).expect("input should be valid");
        let mut analysis =
            KineticsAnalysisResult::analyze(&input).expect("analysis should succeed");
        update(&mut analysis);
        KineticsPlotData::from_analysis(&input, &columns, &analysis)
            .expect("plot data should construct")
    }

    fn standard_data() -> KineticsPlotData {
        plot_data(
            "time",
            "concentration",
            vec![2.0, 0.0, 1.0],
            vec![0.6, 1.0, 0.8],
        )
    }

    #[test]
    fn svg_structure_and_byte_contract_are_fixed() {
        let svg = render_kinetics_svg(&standard_data()).expect("SVG should render");

        assert!(svg.starts_with(ROOT_LINE));
        assert_eq!(
            svg.lines().next().map(|line| format!("{line}\n")),
            Some(ROOT_LINE.to_string())
        );
        assert!(svg.contains("viewBox=\"0 0 960 640\" width=\"960\" height=\"640\""));
        assert!(svg.contains("<title id=\"plot-title\">Kinetics concentration versus time</title>"));
        assert!(svg.contains("<desc id=\"plot-desc\">"));
        assert!(svg.ends_with("</svg>\n"));
        assert!(!svg.ends_with("\n\n"));
        assert!(!svg.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(std::str::from_utf8(svg.as_bytes()).is_ok());
    }

    #[test]
    fn element_and_attribute_order_is_fixed_and_safe() {
        let svg = render_kinetics_svg(&standard_data()).expect("SVG should render");
        let ordered = [
            "<title id=\"plot-title\">",
            "<desc id=\"plot-desc\">",
            "<rect x=\"0\"",
            "<g id=\"plot-grid\">",
            "<g id=\"axes\"",
            "<g id=\"first-order-curves\">",
            "<g id=\"second-order-curves\">",
            "<g id=\"observations\">",
            "<g id=\"axis-labels\"",
            "<g id=\"legend\"",
            "<g id=\"fit-summary\"",
            "<g id=\"visualization-warnings\"",
            "</svg>",
        ];
        let mut previous = 0;
        for needle in ordered {
            let index = svg.find(needle).expect("element should be present");
            assert!(index >= previous, "{needle} should remain ordered");
            previous = index;
        }
        assert!(svg.contains("<circle cx=\"764\" cy=\"96\" r=\"3.5\" fill=\"#111827\"/>"));
        assert!(svg.contains("<polyline points=\""));
        for forbidden in [
            "<script",
            "<style",
            "<image",
            "<foreignObject",
            "<a ",
            "<use",
            "<defs",
            "<filter",
            "<animate",
            "<set",
            "href=",
            "xlink",
            "onload=",
            "data:",
            "<!--",
        ] {
            assert!(!svg.contains(forbidden), "forbidden token: {forbidden}");
        }

        let allowed = [
            "svg", "title", "desc", "g", "line", "polyline", "circle", "text", "rect",
        ];
        for fragment in svg.split('<').skip(1) {
            let fragment = fragment.trim_start_matches('/');
            let name: String = fragment
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
                .collect();
            assert!(
                allowed.contains(&name.as_str()),
                "unexpected element: {name}"
            );
        }
    }

    #[test]
    fn fixed_layout_palette_and_line_identity_are_present() {
        let svg = render_kinetics_svg(&standard_data()).expect("SVG should render");

        assert!(svg.contains("<rect x=\"96\" y=\"72\" width=\"624\" height=\"360\""));
        assert!(svg.contains(
            "<text x=\"408\" y=\"476\" text-anchor=\"middle\" font-size=\"14\">time</text>"
        ));
        assert!(svg.contains("<text x=\"32\" y=\"252\" text-anchor=\"middle\" font-size=\"14\" transform=\"rotate(-90 32 252)\">concentration</text>"));
        assert!(svg.contains("<rect x=\"752\" y=\"72\" width=\"176\" height=\"104\""));
        for color in [
            "#ffffff", "#111827", "#334155", "#cbd5e1", "#005ea8", "#a23b00",
        ] {
            assert!(svg.contains(color));
        }
        assert!(svg.contains("stroke=\"#005ea8\" stroke-width=\"2.5\"/>"));
        assert!(svg.contains("stroke=\"#a23b00\" stroke-width=\"2.5\" stroke-dasharray=\"8 5\"/>"));
        assert!(svg.contains("r=\"3.5\" fill=\"#111827\""));
    }

    #[test]
    fn observations_and_existing_segments_keep_their_orders() {
        let data = standard_data();
        let svg = render_kinetics_svg(&data).expect("SVG should render");
        let first_start = svg.find("<g id=\"first-order-curves\">").unwrap();
        let second_start = svg.find("<g id=\"second-order-curves\">").unwrap();
        let observations_start = svg.find("<g id=\"observations\">").unwrap();
        assert!(first_start < second_start && second_start < observations_start);
        assert_eq!(
            svg[first_start..second_start].matches("<polyline ").count(),
            data.first_order().segments().len()
        );
        assert_eq!(
            svg[second_start..observations_start]
                .matches("<polyline ")
                .count(),
            data.second_order().segments().len()
        );
        let observations_end = svg.find("<g id=\"axis-labels\"").unwrap();
        assert_eq!(
            svg[observations_start..observations_end]
                .matches("<circle ")
                .count(),
            data.observations().len()
        );

        let x_bounds = observation_x_bounds(data.observations()).unwrap();
        let all_y: Vec<_> =
            data.observations()
                .iter()
                .map(|point| point.concentration)
                .chain(
                    data.first_order().segments().iter().flat_map(|segment| {
                        segment.points().iter().map(|point| point.concentration())
                    }),
                )
                .chain(
                    data.second_order().segments().iter().flat_map(|segment| {
                        segment.points().iter().map(|point| point.concentration())
                    }),
                )
                .collect();
        let y_bounds = y_bounds(&all_y).unwrap();
        let circles: Vec<_> = data
            .observations()
            .iter()
            .map(|point| {
                format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"#111827\"/>",
                    format_finite(map_x(point.time, x_bounds).unwrap()).unwrap(),
                    format_finite(map_y(point.concentration, y_bounds).unwrap()).unwrap()
                )
            })
            .collect();
        let indices: Vec<_> = circles
            .iter()
            .map(|circle| svg.find(circle).expect("observation should be present"))
            .collect();
        assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn axis_ranges_ticks_and_mapping_are_deterministic() {
        let observations = [
            crate::KineticsPoint {
                row_index: 0,
                time: 0.0,
                concentration: 1.0,
            },
            crate::KineticsPoint {
                row_index: 1,
                time: 10.0,
                concentration: 2.0,
            },
        ];
        let x = observation_x_bounds(&observations).unwrap();
        assert_eq!(x.min, -0.5);
        assert_eq!(x.max, 10.5);
        let generated = ticks(x).unwrap();
        assert_eq!(generated.len(), 6);
        assert_eq!(generated[0], x.min);
        assert_eq!(generated[5], x.max);
        assert!((generated[1] - 1.7).abs() < 1e-12);
        assert_eq!(map_x(x.min, x).unwrap(), 96.0);
        assert_eq!(map_x(x.max, x).unwrap(), 720.0);

        let y = y_bounds(&[0.5, 2.0]).unwrap();
        assert_eq!(y.min, 0.0);
        assert_eq!(y.max, 2.1);
        assert_eq!(map_y(y.min, y).unwrap(), PLOT_BOTTOM);
        assert_eq!(map_y(y.max, y).unwrap(), 72.0);
    }

    #[test]
    fn axis_range_edge_cases_follow_the_contract() {
        let equal = [
            crate::KineticsPoint {
                row_index: 0,
                time: 2.0,
                concentration: 1.0,
            },
            crate::KineticsPoint {
                row_index: 1,
                time: 2.0,
                concentration: 2.0,
            },
        ];
        assert_eq!(
            observation_x_bounds(&equal).unwrap_err(),
            KineticsSvgRenderError::NonRenderableXRange
        );
        let overflow = [
            crate::KineticsPoint {
                row_index: 0,
                time: -f64::MAX,
                concentration: 1.0,
            },
            crate::KineticsPoint {
                row_index: 1,
                time: f64::MAX,
                concentration: 2.0,
            },
        ];
        assert_eq!(
            observation_x_bounds(&overflow).unwrap_err(),
            KineticsSvgRenderError::NonRenderableXRange
        );
        assert_eq!(
            ticks(Bounds {
                min: 0.0,
                max: f64::INFINITY
            })
            .unwrap_err(),
            KineticsSvgRenderError::NonFiniteTick
        );
        assert_eq!(
            y_bounds(&[1.0, f64::NAN]).unwrap_err(),
            KineticsSvgRenderError::NonRenderableAcceptedYRange
        );

        let zero = y_bounds(&[0.0, 0.0]).unwrap();
        assert_eq!(zero.min, 0.0);
        assert_eq!(zero.max, 1e-12);
        let negative = y_bounds(&[-10.0, -2.0]).unwrap();
        assert_eq!(negative.min, -10.4);
        assert_eq!(negative.max, -1.6);
        let negative_zero_span = y_bounds(&[-2.0, -2.0]).unwrap();
        assert_eq!(negative_zero_span.min, -2.1);
        assert_eq!(negative_zero_span.max, -1.9);
    }

    #[test]
    fn six_ticks_per_axis_are_emitted() {
        let svg = render_kinetics_svg(&standard_data()).unwrap();
        let grid =
            &svg[svg.find("<g id=\"plot-grid\">").unwrap()..svg.find("<g id=\"axes\"").unwrap()];
        assert_eq!(grid.matches("<line ").count(), 12);
        let axes = &svg[svg.find("<g id=\"axes\"").unwrap()
            ..svg.find("<g id=\"first-order-curves\">").unwrap()];
        assert_eq!(axes.matches("<text ").count(), 12);
        assert_eq!(axes.matches("<line ").count(), 14);
    }

    #[test]
    fn labels_are_bounded_escaped_and_reused() {
        let escaped = safe_label("a&b<c>d\"e'f").unwrap();
        assert_eq!(escaped, "a&amp;b&lt;c&gt;d&quot;e&apos;f");
        let long = "界".repeat(49);
        let bounded = safe_label(&long).unwrap();
        assert_eq!(bounded.chars().count(), 48);
        assert!(bounded.ends_with('\u{2026}'));
        assert_eq!(
            bounded.chars().take(47).collect::<String>(),
            "界".repeat(47)
        );

        let data = plot_data("t&<\"'", "c>'&\"", vec![0.0, 1.0, 2.0], vec![1.0, 0.8, 0.6]);
        let svg = render_kinetics_svg(&data).unwrap();
        let time = "t&amp;&lt;&quot;&apos;";
        let concentration = "c&gt;&apos;&amp;&quot;";
        assert_eq!(svg.matches(time).count(), 2);
        assert_eq!(svg.matches(concentration).count(), 2);
        assert!(!svg.contains("t&<\"'"));
        assert!(!svg.contains("onload=\"alert"));
    }

    #[test]
    fn forbidden_label_characters_are_rejected_before_truncation() {
        for character in [
            '\u{1}', '\u{85}', '\u{fffe}', '\u{ffff}', '\u{2028}', '\u{2029}',
        ] {
            let label = format!("safe{character}");
            assert_eq!(
                safe_label(&label).unwrap_err(),
                KineticsSvgRenderError::InvalidLabelCharacter {
                    code_point: character as u32
                }
            );
        }
        let hidden_forbidden = format!("{}\u{1}", "a".repeat(49));
        assert!(matches!(
            safe_label(&hidden_forbidden),
            Err(KineticsSvgRenderError::InvalidLabelCharacter { .. })
        ));
    }

    #[test]
    fn unsafe_labels_cannot_inject_markup_events_or_external_references() {
        let label = "</text><script onload='x'>https://x.invalid";
        let safe = safe_label(label).unwrap();
        assert!(!safe.contains("</text>"));
        assert!(!safe.contains("<script"));
        assert!(!safe.contains("onload='"));
        assert!(safe.contains("https://x.invalid"));
        assert!(safe.contains("&lt;/text&gt;&lt;script onload=&apos;x&apos;&gt;"));
    }

    #[test]
    fn finite_number_formatter_matches_the_byte_contract() {
        for (value, expected) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.25, "1.25"),
            (-0.0001, "-0.0001"),
            (0.000099999, "9.9999e-5"),
            (0.00001, "1e-5"),
            (999_999.0, "999999"),
            (1_000_000.0, "1e+6"),
            (1.23456789, "1.234568"),
            (1.250000, "1.25"),
            (1e-10, "1e-10"),
        ] {
            assert_eq!(format_finite(value).unwrap(), expected);
        }
        assert_eq!(format_finite(-0.0000001).unwrap(), "-1e-7");
        assert_eq!(
            format_finite(f64::NAN).unwrap_err(),
            KineticsSvgRenderError::NonFiniteNumber
        );
        assert_eq!(
            format_finite(f64::INFINITY).unwrap_err(),
            KineticsSvgRenderError::NonFiniteNumber
        );
        for value in [1e-5, 1e6, 1e10] {
            let output = format_finite(value).unwrap();
            assert!(!output.contains('E'));
            assert!(output.contains("e+") || output.contains("e-"));
            let exponent = output.split_once('e').unwrap().1;
            assert!(!exponent.starts_with("+0") && !exponent.starts_with("-0"));
        }
    }

    #[test]
    fn visible_text_is_cautious_accessible_and_bounded() {
        let data = standard_data();
        let svg = render_kinetics_svg(&data).unwrap();
        for required in [
            "role=\"img\" aria-labelledby=\"plot-title plot-desc\"",
            "observed data",
            "first-order fit",
            "second-order fit",
            "first-order r_squared:",
            "second-order r_squared:",
            "MVP heuristic preference:",
            "deterministic review status:",
            "accepted observations: 3",
            "rejected rows: 0",
            "visualization warnings: 0",
            "deterministic review findings: 0",
        ] {
            assert!(svg.contains(required), "missing: {required}");
        }
        for claim in [
            "confirmed reaction order",
            "proven model",
            "correct model",
            "validated mechanism",
            "selected final model",
            "best model",
            "true order",
        ] {
            assert!(!svg.to_lowercase().contains(claim));
        }
    }

    #[test]
    fn rejected_row_warning_is_fixed_and_does_not_leak_row_data() {
        let data = plot_data(
            "time",
            "concentration",
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, -98765.0, 0.7, 0.5],
        );
        let svg = render_kinetics_svg(&data).unwrap();
        assert!(svg.contains("rejected rows: 1"));
        assert!(svg.contains("visualization warnings: 1"));
        assert!(svg.contains(
            "<text x=\"96\" y=\"536\" font-size=\"12\">rejected rows not displayed</text>"
        ));
        assert!(!svg.contains("98765"));
        assert!(!svg.contains("row index"));
        assert!(!svg.contains("rejected_row"));
        assert!(!svg.contains("rejected rows are present and remain visible"));
    }

    #[test]
    fn partial_model_warnings_are_fixed_and_ordered() {
        let data = plot_data_with_analysis(
            "time",
            "concentration",
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, -1.0, 0.8, 0.6],
            |analysis| {
                analysis.comparison.first_order.intercept = 700.0;
                analysis.comparison.first_order.slope = 10.0;
                analysis.comparison.second_order.intercept = -1.0;
                analysis.comparison.second_order.slope = 1.0;
            },
        );
        let svg = render_kinetics_svg(&data).unwrap();
        let rejected = svg.find("rejected rows not displayed").unwrap();
        let first = svg.find("first-order fit partially omitted").unwrap();
        let second = svg.find("second-order fit partially omitted").unwrap();
        assert!(rejected < first && first < second);
        assert!(svg.contains("visualization warnings: 3"));
        assert!(!data.first_order().segments().is_empty());
        assert!(!data.second_order().segments().is_empty());
    }

    #[test]
    fn warning_order_is_rejected_then_first_then_second_and_at_most_three() {
        let data = plot_data_with_analysis(
            "time",
            "concentration",
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, -1.0, 0.8, 0.6],
            |analysis| {
                analysis.comparison.first_order.intercept = 1000.0;
                analysis.comparison.second_order.intercept = -1.0;
                analysis.comparison.second_order.slope = 0.0;
            },
        );
        let svg = render_kinetics_svg(&data).unwrap();
        let rejected = svg.find("rejected rows not displayed").unwrap();
        let first = svg.find("first-order fit omitted").unwrap();
        let second = svg.find("second-order fit omitted").unwrap();
        assert!(rejected < first && first < second);
        assert!(svg.contains("y=\"536\""));
        assert!(svg.contains("y=\"568\""));
        assert!(svg.contains("y=\"600\""));
        assert!(svg.contains("visualization warnings: 3"));
        let warning_group = &svg[svg.find("<g id=\"visualization-warnings\"").unwrap()..];
        assert_eq!(warning_group.matches("<text ").count(), 3);
    }

    #[test]
    fn preferred_model_omission_does_not_change_the_preference_or_review() {
        let data = plot_data_with_analysis(
            "time",
            "concentration",
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.8, 0.6],
            |analysis| {
                analysis.comparison.first_order.intercept = 1000.0;
                analysis.preferred_model = KineticsModelKind::FirstOrder;
                analysis.comparison.preferred_model = KineticsModelKind::FirstOrder;
            },
        );
        let original_status = data.review_status();
        let svg = render_kinetics_svg(&data).unwrap();
        assert!(data.first_order().segments().is_empty());
        assert!(svg.contains("MVP heuristic preference: first-order"));
        assert!(svg.contains(super::review_status_label(original_status)));
        assert!(svg.contains("first-order fit omitted: fewer than two finite predictions"));
    }

    #[test]
    fn all_review_status_wordings_are_stable() {
        assert_eq!(review_status_label(KineticsReviewStatus::Passed), "passed");
        assert_eq!(
            review_status_label(KineticsReviewStatus::PassedWithWarnings),
            "passed with warnings"
        );
        assert_eq!(review_status_label(KineticsReviewStatus::Failed), "failed");
    }

    #[test]
    fn renderer_range_failure_omits_only_the_model() {
        let data = plot_data_with_analysis(
            "time",
            "concentration",
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.8, 0.6],
            |analysis| {
                analysis.comparison.first_order.intercept = f64::MAX.ln();
                analysis.comparison.first_order.slope = 0.0;
                analysis.preferred_model = KineticsModelKind::FirstOrder;
                analysis.comparison.preferred_model = KineticsModelKind::FirstOrder;
            },
        );
        assert!(!data.first_order().segments().is_empty());
        let svg = render_kinetics_svg(&data).unwrap();
        let first_group = &svg[svg.find("<g id=\"first-order-curves\">").unwrap()
            ..svg.find("<g id=\"second-order-curves\">").unwrap()];
        assert!(!first_group.contains("<polyline"));
        assert!(svg.contains("first-order fit omitted: render range not representable"));
        let observations = &svg[svg.find("<g id=\"observations\">").unwrap()
            ..svg.find("<g id=\"axis-labels\"").unwrap()];
        assert_eq!(
            observations.matches("<circle ").count(),
            data.accepted_count()
        );
        assert!(svg.contains("MVP heuristic preference: first-order"));
    }

    #[test]
    fn rendering_is_byte_identical_and_contains_no_environment_metadata() {
        let data = standard_data();
        let first = render_kinetics_svg(&data).unwrap();
        let second = render_kinetics_svg(&data).unwrap();
        assert_eq!(first, second);
        for forbidden in [
            "timestamp",
            "uuid",
            "random",
            "generator",
            "/tmp/",
            "/Users/",
            "C:\\\\",
            "input.csv",
            "output.svg",
            "localhost",
            "http://example",
            "https://example",
        ] {
            assert!(
                !first.contains(forbidden),
                "forbidden metadata: {forbidden}"
            );
        }
    }

    #[test]
    fn private_limit_seam_fails_without_returning_a_truncated_document() {
        let data = standard_data();
        assert_eq!(
            render_with_limit(&data, ROOT_LINE.len() - 1).unwrap_err(),
            KineticsSvgRenderError::SvgSizeExceeded {
                maximum: ROOT_LINE.len() - 1
            }
        );
        let svg = render_kinetics_svg(&data).unwrap();
        assert!(svg.len() < 4 * 1024 * 1024);
    }
}
