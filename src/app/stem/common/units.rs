//! Units module, defining the units used internally within Epsilon and
//! methods for parsing and formatting the user's preferred units.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use uom::fmt::DisplayStyle;
use uom::si::angle;
use uom::si::f64::*;
use uom::si::length;
use uom::si::velocity;
use uom::str::ParseQuantityError;

// Epsilon's internal unit representation
pub type BaseLength = length::meter;
pub type BaseVelocity = velocity::meter_per_second;
pub type BaseAngle = angle::degree;
pub static BASE_ANGLE_INST: BaseAngle = angle::degree;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitPreference {
    // lengths much larger than human scale, e.g. distance traveled
    pub large_length: LengthUnits,
    // lengths comparable to human scale, e.g. gps accuracies, altitude
    pub small_length: LengthUnits,
    pub velocity: VelocityUnits,
}

impl Default for UnitPreference {
    fn default() -> Self {
        Self::imperial_default()
    }
}

impl UnitPreference {
    pub fn metric_default() -> Self {
        Self {
            large_length: LengthUnits::Kilometer,
            small_length: LengthUnits::Meter,
            velocity: VelocityUnits::KilometerPerHour,
        }
    }
    pub fn imperial_default() -> Self {
        Self {
            large_length: LengthUnits::Mile,
            small_length: LengthUnits::Foot,
            velocity: VelocityUnits::MilePerHour,
        }
    }

    pub fn parse_large_length(
        &self,
        val: &str,
    ) -> Result<f64, ParseQuantityError> {
        Self::parse_length(&self.large_length, val)
    }

    pub fn parse_small_length(
        &self,
        val: &str,
    ) -> Result<f64, ParseQuantityError> {
        Self::parse_length(&self.small_length, val)
    }

    pub fn parse_velocity(&self, val: &str) -> Result<f64, ParseQuantityError> {
        // first try to parse as if the unit is specified, e.g. "5 m/s"
        let result = Velocity::from_str(val).map(|x| x.get::<BaseVelocity>());
        if result.is_err() {
            // if failed (e.g. no unit was specified), try to parse as user
            // preference units
            if let Ok(parsed) = val.parse::<f64>() {
                return Ok(self.velocity.to_base_unit(parsed));
            }
        }
        result
    }

    pub fn parse_angle(&self, val: &str) -> Result<f64, ParseQuantityError> {
        let result = Angle::from_str(val).map(|x| x.get::<BaseAngle>());
        if result.is_err() {
            if let Ok(parsed) = val.parse::<f64>() {
                // angles all in degrees, assume that was the unit if not
                // specified
                return Ok(parsed);
            }
        }
        result
    }

    /// Pase a length as user preference units (or specified units) and
    /// convert to Epsilon's internal representation
    fn parse_length(
        preferred_length_unit: &LengthUnits,
        val: &str,
    ) -> Result<f64, ParseQuantityError> {
        let result = Length::from_str(val).map(|x| x.get::<BaseLength>());
        if result.is_err() {
            if let Ok(parsed) = val.parse::<f64>() {
                return Ok(preferred_length_unit.to_base_unit(parsed));
            }
        }
        result
    }

    pub fn format_large_length(&self, val: f64, prec: Option<usize>) -> String {
        self.large_length.format_from_base_unit(val, prec)
    }

    pub fn format_small_length(&self, val: f64, prec: Option<usize>) -> String {
        self.small_length.format_from_base_unit(val, prec)
    }

    pub fn format_velocity(&self, val: f64, prec: Option<usize>) -> String {
        self.velocity.format_from_base_unit(val, prec)
    }

    pub fn format_angle(&self, val: f64, prec: Option<usize>) -> String {
        // just so we don't need to import it separately
        format_angle(val, prec)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LengthUnits {
    Kilometer,
    Meter,
    Foot,
    Mile,
    NauticalMile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VelocityUnits {
    MeterPerSecond,
    KilometerPerHour,
    MilePerHour,
    Knot,
}

pub static LENGTH_STRINGS: [(LengthUnits, &str); 5] = [
    (LengthUnits::Kilometer, "Kilometer"),
    (LengthUnits::Meter, "Meter"),
    (LengthUnits::Foot, "Foot"),
    (LengthUnits::Mile, "Mile"),
    (LengthUnits::NauticalMile, "Nautical Mile"),
];

pub static VELOCITY_STRINGS: [(VelocityUnits, &str); 4] = [
    (VelocityUnits::MeterPerSecond, "Meter per Second"),
    (VelocityUnits::KilometerPerHour, "Kilometer per Hour"),
    (VelocityUnits::MilePerHour, "Mile per Hour"),
    (VelocityUnits::Knot, "Knot"),
];

/// Format a value with an optional precision.
fn format_with_prec<T>(f: T, prec: Option<usize>) -> String
where
    T: std::fmt::Display,
{
    if let Some(p) = prec {
        format!("{:.*}", p, f)
    } else {
        format!("{}", f)
    }
}

impl LengthUnits {
    /// Turn a value in the preferred unit into Epsilon's base unit
    pub fn to_base_unit(&self, val: f64) -> f64 {
        // Haven't figured out how to generically pass uom dimensions and units
        // as values since they're all different types. So everything is
        // contained within individual match arms.
        let quantity = match self {
            Self::Kilometer => Length::new::<length::kilometer>(val),
            Self::Meter => Length::new::<length::meter>(val),
            Self::Foot => Length::new::<length::foot>(val),
            Self::Mile => Length::new::<length::mile>(val),
            Self::NauticalMile => Length::new::<length::nautical_mile>(val),
        };
        quantity.get::<BaseLength>()
    }

    /// Format a base unit value into a string with user preference units
    pub fn format_from_base_unit(
        &self,
        val: f64,
        prec: Option<usize>,
    ) -> String {
        match self {
            Self::Kilometer => Self::do_fmt(length::kilometer, val, prec),
            Self::Meter => Self::do_fmt(length::meter, val, prec),
            Self::Foot => Self::do_fmt(length::foot, val, prec),
            Self::Mile => Self::do_fmt(length::mile, val, prec),
            Self::NauticalMile => {
                Self::do_fmt(length::nautical_mile, val, prec)
            }
        }
    }

    fn do_fmt<N>(unit: N, val: f64, prec: Option<usize>) -> String
    where
        N: length::Unit + uom::Conversion<f64, T = f64>,
    {
        let val = Length::new::<BaseLength>(val);
        let f = Length::format_args(unit, DisplayStyle::Abbreviation).with(val);
        format_with_prec(f, prec)
    }
}

impl VelocityUnits {
    pub fn to_base_unit(&self, val: f64) -> f64 {
        let quantity = match self {
            Self::MeterPerSecond => {
                Velocity::new::<velocity::meter_per_second>(val)
            }
            Self::KilometerPerHour => {
                Velocity::new::<velocity::kilometer_per_hour>(val)
            }
            Self::MilePerHour => Velocity::new::<velocity::mile_per_hour>(val),
            Self::Knot => Velocity::new::<velocity::knot>(val),
        };
        quantity.get::<BaseVelocity>()
    }

    pub fn format_from_base_unit(
        &self,
        val: f64,
        prec: Option<usize>,
    ) -> String {
        match self {
            Self::MeterPerSecond => {
                Self::do_fmt(velocity::meter_per_second, val, prec)
            }
            Self::KilometerPerHour => {
                Self::do_fmt(velocity::kilometer_per_hour, val, prec)
            }
            Self::MilePerHour => {
                Self::do_fmt(velocity::mile_per_hour, val, prec)
            }
            Self::Knot => Self::do_fmt(velocity::knot, val, prec),
        }
    }

    fn do_fmt<N>(unit: N, val: f64, prec: Option<usize>) -> String
    where
        N: velocity::Unit + uom::Conversion<f64, T = f64>,
    {
        let val = Velocity::new::<BaseVelocity>(val);
        let f =
            Velocity::format_args(unit, DisplayStyle::Abbreviation).with(val);
        format_with_prec(f, prec)
    }
}

pub fn format_angle(val: f64, prec: Option<usize>) -> String {
    let style = DisplayStyle::Abbreviation;
    let input = Angle::new::<BaseAngle>(val);
    let f = Angle::format_args(BASE_ANGLE_INST, style).with(input);
    format_with_prec(f, prec)
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEnumError;

impl fmt::Display for LengthUnits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = LENGTH_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

impl std::str::FromStr for LengthUnits {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = LENGTH_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}

impl fmt::Display for VelocityUnits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = VELOCITY_STRINGS.iter().find(|x| x.0 == *self).unwrap();
        write!(f, "{}", item.1)
    }
}

impl std::str::FromStr for VelocityUnits {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let item = VELOCITY_STRINGS
            .iter()
            .find(|x| x.1 == s)
            .ok_or(ParseEnumError)?;
        Ok(item.0.clone())
    }
}
