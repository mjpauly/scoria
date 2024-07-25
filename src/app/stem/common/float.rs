//! Helper functions for dealing with floating point numbers.

/// Floats don't implement Ord, so we have to do this
pub fn min<'a>(vals: impl Iterator<Item = &'a f64>) -> f64 {
    *vals.min_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0)
}

pub fn max<'a>(vals: impl Iterator<Item = &'a f64>) -> f64 {
    *vals.max_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0)
}
