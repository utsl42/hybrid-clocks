use serde::{de, ser};

#[derive(Serialize, Deserialize)]
struct Timestamp<T>(T, u16);

impl<T: ser::Serialize + Copy> ser::Serialize for crate::Timestamp<T> {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self::Timestamp(self.time, self.count).serialize(serializer)
    }
}

impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for crate::Timestamp<T> {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<crate::Timestamp<T>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let self::Timestamp(time, count) = de::Deserialize::deserialize(deserializer)?;
        Ok(crate::Timestamp { time, count })
    }
}
