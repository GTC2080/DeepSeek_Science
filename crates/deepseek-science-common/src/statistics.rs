//! Tiny deterministic statistics helpers.

use crate::CommonError;

/// Computes the arithmetic mean for a non-empty slice.
pub fn mean(values: &[f64]) -> Result<f64, CommonError> {
    if values.is_empty() {
        return Err(CommonError::EmptyInput);
    }

    let sum: f64 = values.iter().sum();
    Ok(sum / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::mean;

    #[test]
    fn mean_works() {
        let result = mean(&[1.0, 2.0, 3.0, 4.0]);

        assert_eq!(result, Ok(2.5));
    }
}
