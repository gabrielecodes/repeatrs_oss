pub mod repeatrs {
    tonic::include_proto!("repeatrs.v1");
}

pub mod timestamp_format {
    use prost_types::Timestamp;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(date: &Option<Timestamp>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(ts) => ts.seconds.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = i64::deserialize(deserializer).ok();
        Ok(seconds.map(|s| Timestamp {
            seconds: s,
            nanos: 0,
        }))
    }
}
