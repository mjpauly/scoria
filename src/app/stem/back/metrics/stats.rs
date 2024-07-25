//! Statistics on iterators of f64.

use itertools::Itertools;

/// Compute a statistic on all lines within a LineString.
pub fn compute_stat<'a, T: 'a, I, V>(
    records: I,
    stat_fn: impl Fn((&T, &T)) -> V,
) -> Vec<V>
where
    I: Iterator<Item = &'a T>,
    V: PartialOrd,
{
    records.tuple_windows::<(_, _)>().map(stat_fn).collect()
}

/// Mean of a slice of values
pub fn mean(data: impl Iterator<Item = f64> + Clone) -> Option<f64> {
    let sum: f64 = data.clone().sum();
    let count = data.count();
    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

/// Weighted mean, where items are passed as (value, weight)
pub fn weighted_mean(
    data: impl Iterator<Item = (f64, f64)> + Clone,
) -> Option<f64> {
    let weighted_sum: f64 = data.clone().map(|(v, w)| v * w).sum();
    let sum_of_weights: f64 = data.map(|(_, w)| w).sum();
    if sum_of_weights > 0.0 {
        Some(weighted_sum / sum_of_weights)
    } else {
        None
    }
}

/// Standard deviation of an iterator of floats.
///
/// This is a baised estimator, since the sample mean covaries with each data
/// point, and so the stddev is smaller than it should be for small N.
pub fn stddev(data: impl Iterator<Item = f64> + Clone) -> Option<f64> {
    match mean(data.clone()) {
        Some(x_bar) => {
            let n = data.clone().count() as f64;
            let variance = data
                .map(|v| {
                    let diff = v - x_bar;
                    diff * diff
                })
                .sum::<f64>()
                / n;
            Some(variance.sqrt())
        }
        _ => None,
    }
}

/// Weighted standard deviation.
pub fn weighted_stddev(
    data: impl Iterator<Item = (f64, f64)> + Clone,
) -> Option<f64> {
    match weighted_mean(data.clone()) {
        Some(x_bar) => {
            weighted_mean(data.map(|(v, w)| ((v - x_bar).powi(2), w)))
                .map(|variance| variance.sqrt())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use common::Location;

    use super::*;
    use crate::metrics::tests::new_empty_location;

    #[test]
    fn stat_test() {
        let mut rec = new_empty_location();
        let mut recs = vec![];
        // longitudes: 0, 1, 8, 27
        // latitudes: 0, 2, 6, 8
        for i in 0..4 {
            rec.longitude = (i * i * i) as f64;
            rec.latitude = (i as f64) * 2.;
            recs.push(rec.clone());
        }

        let stat_fn = |(prev, curr): (&Location, &Location)| {
            // difference between cubes, multiplied by the current latitude
            (curr.longitude - prev.longitude) * curr.latitude
        };

        // stat values: 2, 28, 114
        let stat_vals = compute_stat(recs.iter(), stat_fn);

        let expected_sum = 144.;
        let expected_mean = 48.;
        let expected_std = 47.8609;

        let sum: f64 = stat_vals.iter().sum();
        assert_eq!(expected_sum, sum);

        let mean = mean(stat_vals.iter().copied());
        assert_eq!(Some(expected_mean), mean);

        let std = stddev(stat_vals.iter().copied());
        let diff = (std.unwrap() - expected_std).abs();
        assert!(diff < 0.001);
    }

    #[test]
    fn weighted_stat_test() {
        let vals = [2.0, 28.0, 114.0];
        let weights = [0.6, 0.3, 0.1];

        let expected_weighted_mean = 21.0;
        let expected_weighted_stddev = 33.10891118717135;

        let zipped = vals.iter().copied().zip(weights.iter().copied());
        let w_mean = weighted_mean(zipped.clone()).unwrap();
        let w_stddev = weighted_stddev(zipped).unwrap();
        assert_relative_eq!(expected_weighted_mean, w_mean);
        assert_relative_eq!(expected_weighted_stddev, w_stddev);
    }
}
