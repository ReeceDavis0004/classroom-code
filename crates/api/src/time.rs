use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serializer;

pub fn ms<S: Serializer>(t: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&t.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub fn ms_opt<S: Serializer>(t: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
    match t {
        Some(t) => ms(t, s),
        None => s.serialize_none(),
    }
}
