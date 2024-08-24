use std::fmt;
use std::fmt::Display;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal::prelude::{FromPrimitive, Zero};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around [`rust_decimal::Decimal`] that serializes and deserializes with four decimal places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Decimal4(Decimal);

const ROUNDING_STRATEGY: RoundingStrategy = RoundingStrategy::MidpointTowardZero;

impl Decimal4 {
    pub fn zero() -> Self {
        Decimal4(Decimal::zero())
    }
}

impl Display for Decimal4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl From<Decimal> for Decimal4 {
    fn from(value: Decimal) -> Self {
        Decimal4(value.round_dp_with_strategy(4, ROUNDING_STRATEGY))
    }
}

impl Into<Decimal> for Decimal4 {
    fn into(self) -> Decimal {
        self.0
    }
}

impl From<i32> for Decimal4 {
    fn from(value: i32) -> Self {
        Decimal4(Decimal::from(value))
    }
}

impl From<u32> for Decimal4 {
    fn from(value: u32) -> Self {
        Decimal4(Decimal::from(value))
    }
}

impl TryFrom<f32> for Decimal4 {
    type Error = rust_decimal::Error;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        let decimal = Decimal::from_f32(value)
            .ok_or(rust_decimal::Error::ErrorString("failed to parse f32".to_string()))?;
        Ok(Decimal4::from(decimal))
    }
}

impl FromStr for Decimal4 {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decimal = Decimal::from_str(s)?;
        Ok(Decimal4::from(decimal))
    }
}

impl Serialize for Decimal4 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Decimal4 {
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
        where T: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Decimal4::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Add for Decimal4 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Decimal4::from(self.0 + other.0)
    }
}

impl Sub for Decimal4 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Decimal4::from(self.0 - other.0)
    }
}

impl AddAssign for Decimal4 {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Decimal4 {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

#[cfg(test)]
mod decimal4_tests {
    use super::*;

    #[test]
    fn decimal4_serialization_keeps_four_digits() {
        assert_eq!(Ok("1.0000".to_string()), Decimal4::from_str("1").map(|x| x.to_string()));
        assert_eq!(Ok("1.0000".to_string()), Decimal4::from_str("1.0000").map(|x| x.to_string()));
        assert_eq!(Ok("1.0100".to_string()), Decimal4::from_str("1.01").map(|x| x.to_string()));
    }

    #[test]
    fn decimal4_deserialization_with_round_up() {
        assert_eq!(Ok("1.2345".to_string()), Decimal4::from_str("1.2345").map(|x| x.to_string()));
        assert_eq!(Ok("1.2346".to_string()), Decimal4::from_str("1.234567").map(|x| x.to_string()));
        assert_eq!(Ok("1.2346".to_string()), Decimal4::from_str("1.23456789").map(|x| x.to_string()));
    }

    #[test]
    fn decimal4_deserialization_with_round_down() {
        assert_eq!(Ok("1.2345".to_string()), Decimal4::from_str("1.2345").map(|x| x.to_string()));
        assert_eq!(Ok("1.2345".to_string()), Decimal4::from_str("1.234543").map(|x| x.to_string()));
        assert_eq!(Ok("1.2345".to_string()), Decimal4::from_str("1.23454321").map(|x| x.to_string()));
    }

    #[test]
    fn decimal4_addition() {
        let a = Decimal4::from_str("1.2345").unwrap();
        let b = Decimal4::from_str("2.3456").unwrap();
        let c = a + b;
        assert_eq!("3.5801".to_string(), c.to_string());
    }

    #[test]
    fn decimal4_subtraction() {
        let a = Decimal4::from_str("1.2345").unwrap();
        let b = Decimal4::from_str("2.3456").unwrap();
        let c = a - b;
        assert_eq!("-1.1111".to_string(), c.to_string());
    }
}
