use std::convert::TryInto;
use std::fmt;
use std::ops::Sub;
use std::time::{Duration, SystemTime};

use super::{ClockSource, NANOS_PER_SEC};
use crate::{Result, Timestamp};

// A clock source that returns wall-clock in 2^(-16)s
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WallMS;
/// Representation of our timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct WallMST(u32, u16);

impl Timestamp<WallMST> {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut res = [0; 8];
        res[0..4].copy_from_slice(&self.time.0.to_be_bytes());
        res[4..6].copy_from_slice(&self.time.1.to_be_bytes());
        res[6..8].copy_from_slice(&self.count.to_be_bytes());
        return res;
    }

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        let secs = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let fraction = u16::from_be_bytes(bytes[4..6].try_into().unwrap());
        let count = u16::from_be_bytes(bytes[6..8].try_into().unwrap());
        Timestamp {
            time: WallMST(secs, fraction),
            count,
        }
    }
}

impl WallMST {
    /// The number of ticks per seconds: 2^(-16).
    pub const TICKS_PER_SEC: u64 = 1 << 16;
    /// 2020-02-20T00:00:00-00:00
    pub const EPOCH_2020: u64 = 1582156800;
    /// Returns the `Duration` since the unix epoch.
    pub fn duration_since_epoch(self) -> Result<Duration> {
        // TODO: use Duration::from_nanos
        let nanos_per_tick = NANOS_PER_SEC / Self::TICKS_PER_SEC;
        let secs = self.0 as u64 + Self::EPOCH_2020;
        let minor_ticks = self.1 as u64;
        let nsecs = minor_ticks * nanos_per_tick;
        assert!(nsecs < 1000_000_000, "Internal arithmetic error");
        Ok(Duration::new(secs as u64, nsecs.try_into().expect("internal error")))
    }

    /// Returns a `SystemTime` representing this timestamp.
    pub fn as_systemtime(self) -> Result<SystemTime> {
        Ok(SystemTime::UNIX_EPOCH + self.duration_since_epoch()?)
    }

    /// Returns a `WallMST` representing the `SystemTime`.
    pub fn from_timespec(t: SystemTime) -> Result<Self> {
        // TODO: use Duration::as_nanos
        let since_epoch = t.duration_since(SystemTime::UNIX_EPOCH)?;
        Self::from_since_epoch(since_epoch)
    }

    /// Returns a `WallMST` from a `Duration` since the unix epoch.
    pub fn from_since_epoch(since_epoch: Duration) -> Result<Self> {
        let nanos_per_tick = crate::source::NANOS_PER_SEC / WallMST::TICKS_PER_SEC;
        let ticks = (since_epoch.as_secs() * WallMST::TICKS_PER_SEC) + (since_epoch.subsec_nanos() as u64/nanos_per_tick);
        Ok(WallMST::of_u64(ticks))
    }

    /// Returns the number of ticks since the unix epoch.
    fn as_u64(self) -> u64 {
        ((self.0 as u64 + Self::EPOCH_2020) * Self::TICKS_PER_SEC) + self.1 as u64
    }

    /// Builds a WallMST from the number of ticks since the unix epoch.
    fn of_u64(val: u64) -> Self {
        let secs = (val >> 16).checked_sub(Self::EPOCH_2020).unwrap_or(0) as u32;
        let minor_ticks = (val % Self::TICKS_PER_SEC) as u16;
        WallMST(secs, minor_ticks)
    }
}

impl Sub for WallMST {
    type Output = Duration;
    fn sub(self, rhs: Self) -> Self::Output {
        let ticks :u64 = (self.as_u64().checked_sub(rhs.as_u64())).expect("inside time range")
            .checked_mul(NANOS_PER_SEC / Self::TICKS_PER_SEC)
            .expect("inside time range");
        Duration::from_nanos(ticks)
    }
}

impl ClockSource for WallMS {
    type Time = WallMST;
    type Delta = Duration;
    fn now(&mut self) -> Result<Self::Time> {
        WallMST::from_timespec(SystemTime::now())
    }
}

impl fmt::Display for WallMST {
    #[cfg(not(feature = "pretty-print"))]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.duration_since_epoch() {
            Ok(epoch) => write!(fmt, "{}", epoch.as_secs_f64()),
            Err(e) => write!(fmt, "{}", e),
        }
    }

    #[cfg(feature = "pretty-print")]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_systemtime() {
            Ok(ts) => {
                let st = time::PrimitiveDateTime::from(ts);
                write!(
                    fmt,
                    "{}.{:09}Z",
                    st.format("%Y-%m-%dT%H:%M:%S"),
                    st.nanosecond(),
                )
            }
            Err(e) => write!(fmt, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WallMST;
    use crate::tests::timestamps;
    use crate::Timestamp;
    use suppositions::generators::*;

    use suppositions::*;

    fn wallclocks2() -> Box<dyn GeneratorObject<Item = WallMST>> {
        u64s()
            .map(|val| {
                let limit = u64::max_value() >> 16;
                let scaled = val & limit;
                eprintln!("{}", WallMST::of_u64(scaled as u64));
                WallMST::of_u64(scaled as u64)
            })
            .boxed()
    }

    #[test]
    fn should_round_trip_via_key() {
        property(timestamps(wallclocks2())).check(|ts| {
            let bs = ts.to_bytes();
            let ts2 = Timestamp::<WallMST>::from_bytes(bs);
            // println!("{:?}\t{:?}", ts == ts2, bs);
            ts == ts2
        });
    }

    #[test]
    fn should_round_trip_via_timespec() {
        // We expect millisecond precision, so ensure we're within ± 0.5ms
        let allowable_error :u128 = (WallMST::TICKS_PER_SEC / 1000 / 2) as u128;

        property(wallclocks2()).check(|wc| {
            let tsp = wc.as_systemtime().expect("wall time");
            let wc2 = WallMST::from_timespec(tsp).expect("from time");
            let diff = (wc - wc2).as_nanos();
            assert!(
                diff <= allowable_error,
                "left:{:#?}; tsp: {:?}; right:{:#?}; diff:{}",
                wc,
                tsp,
                wc2,
                diff
            );
        });
    }

    #[test]
    fn timespec_should_order_as_timestamps() {
        property((wallclocks2(), wallclocks2())).check(|(ta, tb)| {
            use std::cmp::Ord;

            let ba = ta.as_systemtime().expect("wall time");
            let bb = tb.as_systemtime().expect("wall time");
            ta.cmp(&tb) == ba.cmp(&bb)
        })
    }

    #[test]
    fn byte_repr_should_order_as_timestamps() {
        property((timestamps(wallclocks2()), timestamps(wallclocks2()))).check(|(ta, tb)| {
            use std::cmp::Ord;

            let ba = ta.to_bytes();
            let bb = tb.to_bytes();
            ta.cmp(&tb) == ba.cmp(&bb)
        })
    }
}
