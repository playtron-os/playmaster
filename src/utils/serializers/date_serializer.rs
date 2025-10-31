use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

const ISO_FORMAT: &str = "%+";
const DATE_ONLY_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

// The signature of a serialize_with function must follow the pattern:
//
//    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
//    where
//        S: Serializer
//
// although it may also be generic over the input types T.
pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", date.format(ISO_FORMAT));
    serializer.serialize_str(&s)
}

// The signature of a deserialize_with function must follow the pattern:
//
//    fn deserialize<'de, D>(D) -> Result<T, D::Error>
//    where
//        D: Deserializer<'de>
//
// although it may also be generic over the output types T.
#[allow(dead_code)]
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    deserialize_date::<'de, D>(s)
}

#[allow(dead_code)]
fn deserialize_date<'de, D>(s: String) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let dt = NaiveDateTime::parse_from_str(&s, ISO_FORMAT).or_else(|_| {
        NaiveDateTime::parse_from_str(&s, DATE_ONLY_FORMAT).map_err(serde::de::Error::custom)
    })?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
}
