use serde::Serialize;

/// Struct to serialize for updating a ScatterMapbox plot with new data.
#[derive(Serialize, Clone, Debug)]
pub struct ScatterMapboxUpdate<Lat, Lon>
where
    Lat: Serialize + Clone,
    Lon: Serialize + Clone,
{
    // plotly requires an extra level of array nesting, to correspond to the
    // trace update indices
    pub lat: Vec<Vec<Lat>>,
    pub lon: Vec<Vec<Lon>>,
}

impl<Lat, Lon> ScatterMapboxUpdate<Lat, Lon>
where
    Lat: Serialize + Clone,
    Lon: Serialize + Clone,
{
    // pub fn new(lat: Vec<Lat>, lon: Vec<Lon>, marker: Marker) -> Box<Self> {
    pub fn new(lat: Vec<Lat>, lon: Vec<Lon>) -> Box<Self> {
        Box::new(Self {
            lat: vec![lat],
            lon: vec![lon],
        })
    }
}

impl<Lat, Lon> plotly::Trace for ScatterMapboxUpdate<Lat, Lon>
where
    Lat: Serialize + Clone,
    Lon: Serialize + Clone,
{
    fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}
