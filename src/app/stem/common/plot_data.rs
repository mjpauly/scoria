use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TimeSeriesPlot {
    pub t: Vec<time::OffsetDateTime>,
    pub y: Vec<f64>,
    pub ylabel: String,
}
