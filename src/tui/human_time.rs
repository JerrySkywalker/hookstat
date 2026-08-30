//! Deterministic Human-time formatting for normal TUI surfaces.
//!
//! Analytics and machine output retain epoch milliseconds. This module is the
//! sole presentation conversion point so local Human text never falls back to
//! a raw epoch value.

use super::localization::{MessageKey, ResolvedLocale, t};
use chrono::{Local, TimeZone};

pub(super) fn format_human_time(locale: ResolvedLocale, unix_ms: i64, now_unix_ms: i64) -> String {
    format_human_time_in_timezone(locale, unix_ms, now_unix_ms, Local)
}

fn format_human_time_in_timezone<Tz>(
    locale: ResolvedLocale,
    unix_ms: i64,
    now_unix_ms: i64,
    timezone: Tz,
) -> String
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    let Some(local) = timezone.timestamp_millis_opt(unix_ms).single() else {
        return unavailable_time(locale).to_owned();
    };
    format!(
        "{} ({})",
        local.format("%Y-%m-%d %H:%M"),
        relative_age(locale, unix_ms, now_unix_ms)
    )
}

fn relative_age(locale: ResolvedLocale, unix_ms: i64, now_unix_ms: i64) -> String {
    let delta_ms = now_unix_ms.saturating_sub(unix_ms);
    let seconds = delta_ms / 1_000;
    if seconds < 60 {
        return t(locale, MessageKey::TimeJustNow).to_owned();
    }
    let (count, singular, plural) = if seconds < 60 * 60 {
        (
            seconds / 60,
            MessageKey::TimeMinuteAgo,
            MessageKey::TimeMinutesAgo,
        )
    } else if seconds < 24 * 60 * 60 {
        (
            seconds / (60 * 60),
            MessageKey::TimeHourAgo,
            MessageKey::TimeHoursAgo,
        )
    } else {
        (
            seconds / (24 * 60 * 60),
            MessageKey::TimeDayAgo,
            MessageKey::TimeDaysAgo,
        )
    };
    if count == 1 {
        t(locale, singular).to_owned()
    } else {
        t(locale, plural).replace("{count}", &count.to_string())
    }
}

const fn unavailable_time(locale: ResolvedLocale) -> &'static str {
    t(locale, MessageKey::TimeUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn east_eight() -> FixedOffset {
        FixedOffset::east_opt(8 * 60 * 60).unwrap()
    }

    #[test]
    fn formats_local_datetime_with_deterministic_relative_age() {
        let offset = east_eight();
        let now = offset
            .with_ymd_and_hms(2026, 8, 30, 9, 34, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let timestamp = offset
            .with_ymd_and_hms(2026, 8, 30, 9, 22, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            format_human_time_in_timezone(ResolvedLocale::EnUs, timestamp, now, offset),
            "2026-08-30 09:22 (12 minutes ago)"
        );
    }

    #[test]
    fn relative_age_boundaries_are_localized_without_epoch_fallback() {
        let offset = east_eight();
        let now = offset
            .with_ymd_and_hms(2026, 8, 30, 9, 22, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert!(
            format_human_time_in_timezone(ResolvedLocale::EnUs, now - 59_000, now, offset)
                .ends_with("(just now)")
        );
        assert!(
            format_human_time_in_timezone(ResolvedLocale::ZhCn, now - 60_000, now, offset)
                .ends_with("(1 分钟前)")
        );
        assert!(
            format_human_time_in_timezone(ResolvedLocale::EnUs, i64::MAX, now, offset)
                .contains("Time unavailable")
        );
    }
}
