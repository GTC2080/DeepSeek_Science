//! Tiny deterministic statistics helpers.

use crate::CommonError;

/// Computes the arithmetic mean for finite values in a non-empty slice.
pub fn mean(values: &[f64]) -> Result<f64, CommonError> {
    if values.is_empty() {
        return Err(CommonError::EmptyInput);
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(CommonError::NonFiniteValue);
    }

    let sum: f64 = values.iter().sum();
    Ok(sum / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use crate::CommonError;

    use super::mean;

    #[test]
    fn mean_works_for_simple_finite_values() {
        let result = mean(&[1.0, 2.0, 3.0, 4.0]);

        assert_eq!(result, Ok(2.5));
    }

    #[test]
    fn mean_rejects_empty_input() {
        let result = mean(&[]);

        assert_eq!(result, Err(CommonError::EmptyInput));
    }

    #[test]
    fn mean_rejects_non_finite_values() {
        let nan_result = mean(&[1.0, f64::NAN]);
        let infinity_result = mean(&[1.0, f64::INFINITY]);

        assert_eq!(nan_result, Err(CommonError::NonFiniteValue));
        assert_eq!(infinity_result, Err(CommonError::NonFiniteValue));
    }
}
