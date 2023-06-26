//! Helper functions for dealing with floating point numbers.

/// Floats don't implement Ord, so we have to do this
pub fn min(vals: &[f64]) -> f64 {
    *vals.iter().min_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0)
}

pub fn max(vals: &[f64]) -> f64 {
    *vals.iter().max_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0)
}
