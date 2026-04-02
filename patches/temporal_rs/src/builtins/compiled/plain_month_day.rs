use crate::{
  PlainMonthDay, TemporalResult, TimeZone, builtins::TZ_PROVIDER, unix_time::EpochNanoseconds,
};

impl PlainMonthDay {
  pub fn epoch_ns_for(&self, time_zone: TimeZone) -> TemporalResult<EpochNanoseconds> {
    self.epoch_ns_for_with_provider(time_zone, &*TZ_PROVIDER)
  }
}
