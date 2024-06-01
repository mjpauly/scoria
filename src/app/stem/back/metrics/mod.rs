//! Higher-level analysis of logged location data. Core feature of Scoria
//! which allows people to extract useful information from their movement data.

pub mod dashboard; // stats dashboard

// segment LineStrings into MultiLineStrings
pub mod segmentation;

pub mod stats; // sum, mean, count, stddev, etc

pub mod distance; // Computing distance between data points
pub mod speed;

pub mod color; // calculate colormaps for data

#[cfg(test)]
pub mod tests {
    pub fn new_empty_location() -> common::Location {
        common::Location {
            timestamp: time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
            latitude: 0.,
            longitude: 0.,
            horizontal_accuracy: 5.,
            msl_altitude: None,
            ellipsoid_altitude: None,
            vertical_accuracy: None,
            story: None,
            speed: None,
            speed_accuracy: None,
            course: None,
            course_accuracy: None,
            is_simulated_by_software: None,
            is_produced_by_accessory: None,
            was_imported: false,
        }
    }
}
