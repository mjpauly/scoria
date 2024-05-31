//! Statistics on contiguous LineStrings.

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

/// Total of a slice of values
pub fn stat_sum(data: &[f64]) -> f64 {
    data.iter().sum()
}

/// Count of a slice of values
pub fn stat_count(data: &[f64]) -> usize {
    data.len()
}

/// Mean of a slice of values
pub fn stat_mean(data: &[f64]) -> Option<f64> {
    let s = stat_sum(data);
    let c = stat_count(data);
    match c {
        positive if positive > 0 => Some(s / c as f64),
        _ => None,
    }
}

/// Standard deviation of a slice of values
#[allow(unused)]
fn stat_std(data: &[f64]) -> Option<f64> {
    match (stat_mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - value;
                    diff * diff
                })
                .sum::<f64>()
                / count as f64;
            Some(variance.sqrt())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_stat, stat_mean, stat_std, stat_sum};
    use crate::metrics::tests::new_empty_location;
    use common::Location;

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

        let sum = stat_sum(&stat_vals);
        assert_eq!(expected_sum, sum);

        let mean = stat_mean(&stat_vals);
        assert_eq!(Some(expected_mean), mean);

        let std = stat_std(&stat_vals);
        let diff = (std.unwrap() - expected_std).abs();
        assert!(diff < 0.001);
    }
}
