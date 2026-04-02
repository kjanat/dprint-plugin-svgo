use crate::{
  TemporalResult, TimeZone,
  builtins::TZ_PROVIDER,
  builtins::core::{PlainDateTime, ZonedDateTime},
  options::Disambiguation,
};

impl PlainDateTime {
  /// Returns a `ZonedDateTime` with the provided `PlainDateTime`, TimeZone` and
  /// `Disambiguation`.
  ///
  /// # Feature gated
  ///
  /// Enable with the `compiled_data` feature flag.
  pub fn to_zoned_date_time(
    &self,
    time_zone: TimeZone,
    disambiguation: Disambiguation,
  ) -> TemporalResult<ZonedDateTime> {
    self.to_zoned_date_time_with_provider(time_zone, disambiguation, &*TZ_PROVIDER)
  }
}

#[cfg(test)]
mod tests {
  #[cfg(feature = "tzdb")]
  #[test]
  fn to_zoned_date_time_edge_cases() {
    use crate::{PlainDateTime, TimeZone, options::Disambiguation, tzdb::CompiledTzdbProvider};
    let provider = &CompiledTzdbProvider::default();

    // Test that a non existent PlainDateTime is successfully disambiguated.
    //
    // NOTE(nekevss): POSIX time zone logic of the underlying provider if TZDB is in a "slim" format.
    let pdt = PlainDateTime::try_new_iso(2020, 3, 8, 2, 30, 0, 0, 0, 0).unwrap();
    let zdt = pdt
      .to_zoned_date_time_with_provider(
        TimeZone::try_from_identifier_str_with_provider("America/Los_Angeles", provider).unwrap(),
        Disambiguation::Compatible,
        provider,
      )
      .unwrap();
    // Should disambiguate to 2020-03-08T01:30:00-08:00[America/Los_Angeles]
    assert_eq!(zdt.hour(), 3);
  }
}
