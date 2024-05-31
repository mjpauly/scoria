//! Segment LineStrings into multiple line strings, such as whenever some
//! threshold is crossed.

use itertools::Itertools;

/// Segments a LineString into two MultiLineStrings, one for the contiguous
/// LineStrings above the threshold and one for below (or equal).
///
/// Records serve as the boundaries between segments, and are shared between
/// adjacent segments. This is because datapoints are just points in time,
/// whereas the timespan between them allows for fully segmenting all possible
/// time values. When the threshold is crossed, the datapoint at the boundary
/// appears in both of the LineStrings that are added to `above` and `below`.
///
/// Returns two vectors of vectors containing references to records, the first
/// for records above the threhsold, and the second below or equal.
///
/// # Example
///
/// `|` is a record
/// numbers are the computed value on the segment
/// threshold is 0.5
///
/// input:
///   |__0__|__1__|__0__|__0__|__1__|__1__|__0__|
///
/// output:
/// above:
/// [       |__1__|     ,     |__1__|__1__|       ]
/// below:
/// [ |__0__|  ,  |__0__|__0__|     ,     |__0__| ]
///
/// # Function Signature
///
/// Generic parameters:
///     T: a record, such as a location
///     I: an iterator over references to records
///     V: the type of the value to compare to determine where a segment goes
pub fn threshold_segmentation<'a, T, I, V>(
    records: I,
    segment_fn: impl Fn(&(&T, &T)) -> V,
    threshold: V,
) -> (Vec<Vec<&'a T>>, Vec<Vec<&'a T>>)
where
    I: Iterator<Item = &'a T>,
    V: PartialOrd,
{
    let mut above = vec![]; // MultiLineString for records above the threshold
    let mut below = vec![];
    for (is_above, mut chunk) in &records
        .tuple_windows::<(_, _)>()
        .chunk_by(|x| segment_fn(x) > threshold)
    {
        let first = chunk.next().unwrap(); // chunks have at least one element
        let mut unpaired = vec![first.0, first.1];
        unpaired.extend(chunk.map(|p| p.1));
        if is_above {
            above.push(unpaired);
        } else {
            below.push(unpaired);
        }
    }
    (above, below)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_iter_test() {
        let arr = (1..4).collect::<Vec<_>>();
        let mut pair_iter = arr.iter().tuple_windows();
        assert_eq!(pair_iter.next(), Some((&1, &2)));
        assert_eq!(pair_iter.next(), Some((&2, &3)));
        assert_eq!(pair_iter.next(), None);
    }
}
