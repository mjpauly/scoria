//! Types common to the front and back ends

/// Struct representation of a Location data point
#[derive(Clone, FromRow, Debug)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
    pub course: f64,
    pub datetime: time::OffsetDateTime, // OffsetDateTime is timezone aware
}
