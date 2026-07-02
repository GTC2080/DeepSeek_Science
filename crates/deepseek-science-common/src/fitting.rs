//! Small fitting helpers for future workflow prototypes.

use crate::{mean, CommonError};
use serde::{Deserialize, Serialize};

/// Result of an ordinary least-squares line fit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearRegression {
    /// Fitted slope.
    pub slope: f64,
    /// Fitted intercept.
    pub intercept: f64,
    /// Coefficient of determination.
    pub r_squared: f64,
}

/// Fits `y = slope * x + intercept` over paired slices.
pub fn simple_linear_regression(
    x_values: &[f64],
    y_values: &[f64],
) -> Result<LinearRegression, CommonError> {
    if x_values.len() != y_values.len() {
        return Err(CommonError::LengthMismatch);
    }
    if x_values.len() < 2 {
        return Err(CommonError::TooFewObservations);
    }

    let x_mean = mean(x_values)?;
    let y_mean = mean(y_values)?;
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    let mut total_sum_squares = 0.0;

    for (&x, &y) in x_values.iter().zip(y_values.iter()) {
        let x_delta = x - x_mean;
        numerator += x_delta * (y - y_mean);
        denominator += x_delta * x_delta;
        total_sum_squares += (y - y_mean) * (y - y_mean);
    }

    if denominator == 0.0 {
        return Err(CommonError::ZeroVariance);
    }

    let slope = numerator / denominator;
    let intercept = y_mean - slope * x_mean;
    let mut residual_sum_squares = 0.0;

    for (&x, &y) in x_values.iter().zip(y_values.iter()) {
        let predicted = slope * x + intercept;
        residual_sum_squares += (y - predicted) * (y - predicted);
    }

    let r_squared = if total_sum_squares == 0.0 {
        1.0
    } else {
        1.0 - residual_sum_squares / total_sum_squares
    };

    Ok(LinearRegression {
        slope,
        intercept,
        r_squared,
    })
}

#[cfg(test)]
mod tests {
    use super::simple_linear_regression;

    #[test]
    fn linear_regression_works_on_simple_line() {
        let result = simple_linear_regression(&[0.0, 1.0, 2.0, 3.0], &[1.0, 3.0, 5.0, 7.0]);

        match result {
            Ok(fit) => {
                assert!((fit.slope - 2.0).abs() < f64::EPSILON);
                assert!((fit.intercept - 1.0).abs() < f64::EPSILON);
                assert!((fit.r_squared - 1.0).abs() < f64::EPSILON);
            }
            Err(error) => panic!("linear regression should fit simple data: {error}"),
        }
    }
}
