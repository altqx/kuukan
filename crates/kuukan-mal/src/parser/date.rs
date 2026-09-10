//! Port of the date helpers in `Jikan\Helper\Parser` (jikan-php v4.0.12).
//!
//! Every function returns a `chrono::DateTime<FixedOffset>` (PHP
//! `\DateTimeImmutable`), `None` where PHP returns `null`. Values parsed
//! without an explicit offset are anchored to UTC, exactly like
//! `new \DateTimeImmutable($date, new \DateTimeZone('UTC'))`.
//!
//! Note: PHP raises a `TypeError` when `DateTimeImmutable::createFromFormat()`
//! returns `false` (declared `?DateTimeImmutable`). Kuukan returns `None` in
//! those cases, which is the closest non-panicking equivalent.

use chrono::{
    DateTime, Datelike, Days, Duration, FixedOffset, Months, NaiveDate, NaiveDateTime, TimeZone,
    Utc, Weekday,
};
use regex::Regex;
use std::fmt::Display;
use std::sync::OnceLock;

/// PHP `DATE_ATOM`: `Y-m-d\TH:i:sP`, e.g. `2024-01-06T00:00:00+00:00`.
pub fn format_atom<Tz>(dt: &DateTime<Tz>) -> String
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Port of `Parser::parseDate()`.
pub fn parse_date(date: &str) -> Option<DateTime<FixedOffset>> {
    // if (preg_match('/^\d{4}$/', $date))
    if date.len() == 4 && date.bytes().all(|b| b.is_ascii_digit()) {
        let year: i32 = date.parse().ok()?;
        return utc_date(year, 1, 1);
    }

    // if (preg_match('/(\w{3}), (\d{4})/', $date, $matches))
    // createFromFormat('!M d, Y', "{$matches[1]} 01, {$matches[2]}")
    if let Some(caps) = month_comma_year_re().captures(date) {
        let month = month_number(&caps[1])?;
        let year: i32 = caps[2].parse().ok()?;
        return utc_date(year, month, 1);
    }

    // new \DateTimeImmutable($date, new \DateTimeZone('UTC')) in try/catch
    parse_php_datetime(date)
}

/// Port of `Parser::parseForumDate()`.
pub fn parse_forum_date(date: &str) -> Option<DateTime<FixedOffset>> {
    let mut date = date.to_string();
    if !four_digits_re().is_match(&date) {
        date = format!("{}, {}", date, Utc::now().year());
    }
    parse_date(&date)
}

/// Port of `Parser::parseDateMDY()` (`!m-d-y`).
pub fn parse_date_mdy(date: Option<&str>) -> Option<DateTime<FixedOffset>> {
    let date = date?;
    if date == "-" {
        return None;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() >= 3
        && parts[0] == "??"
        && parts[1] == "??"
        && parts[2] == current_two_digit_year()
    {
        return None;
    }
    let date = date.replace("??", "01");
    create_from_format_mdy(&date)
}

/// Port of `Parser::parseDateDMY()` (`!d-m-y`).
pub fn parse_date_dmy(date: Option<&str>) -> Option<DateTime<FixedOffset>> {
    let date = date?;
    if date == "-" {
        return None;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() >= 3
        && parts[0] == "??"
        && parts[1] == "??"
        && parts[2] == current_two_digit_year()
    {
        return None;
    }
    let date = date.replace("??", "01");
    create_from_format_dmy(&date)
}

/// Port of `Parser::parseDateMDYReadable()`.
///
/// PHP does not catch the `DateTimeImmutable` constructor here (the exception
/// propagates to `MalClient`); Kuukan degrades to `None`.
pub fn parse_date_mdy_readable(date: &str) -> Option<DateTime<FixedOffset>> {
    let date = date.replace("  ", " ");

    // if (preg_match('~[a-zA-z]+ \d+, \d{4}~', $date))
    if readable_date_re().is_match(&date) {
        return parse_php_datetime(&date);
    }

    // if (preg_match('~^([a-zA-z]+), (\d{4})$~', $date, $matches))
    if let Some(caps) = month_comma_year_anchored_re().captures(&date) {
        let reconstructed = format!("{} 01, {}", &caps[1], &caps[2]);
        return parse_php_datetime(&reconstructed);
    }

    None
}

/// Port of `Parser::parseDateTimePST()`.
///
/// The MAL timestamp is a wall clock in `America/Los_Angeles`; the result is
/// converted to UTC. chrono has no timezone database, so the post-2007 US DST
/// rule is applied manually (second Sunday of March .. first Sunday of
/// November; pre-2007: first Sunday of April .. last Sunday of October).
pub fn parse_date_time_pst(date_time: &str) -> Option<DateTime<FixedOffset>> {
    let naive = parse_naive_datetime(date_time)?;
    let offset = la_utc_offset_seconds(naive);
    let utc = naive - Duration::seconds(offset as i64);
    Some(DateTime::from_naive_utc_and_offset(
        utc,
        FixedOffset::east_opt(0).expect("UTC offset"),
    ))
}

/// Port of `Parser::parseDurationToSeconds()`.
pub fn parse_duration_to_seconds(duration: &str) -> Option<i64> {
    let caps = duration_re().captures(duration)?;
    let hours: i64 = caps[1].parse().ok()?;
    let minutes: i64 = caps[2].parse().ok()?;
    let seconds: i64 = caps[3].parse().ok()?;
    Some(hours * 3600 + minutes * 60 + seconds)
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

fn utc_date(year: i32, month: u32, day: u32) -> Option<DateTime<FixedOffset>> {
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    Some(DateTime::from_naive_utc_and_offset(
        date.and_hms_opt(0, 0, 0)?,
        FixedOffset::east_opt(0)?,
    ))
}

fn current_two_digit_year() -> String {
    format!("{:02}", Utc::now().year().rem_euclid(100))
}

/// `DateTimeImmutable::createFromFormat('!m-d-y', ...)`.
///
/// PHP's parser accepts 1-2 digits per field and normalizes overflows
/// (`13-01-21` -> 2022-01-01, `02-30-21` -> 2021-03-02, `00-00-00` ->
/// 1999-11-30). Two-digit years: 00-69 -> 2000-2069, 70-99 -> 1970-1999.
fn create_from_format_mdy(date: &str) -> Option<DateTime<FixedOffset>> {
    let caps = mdy_re().captures(date)?;
    let month: u32 = caps[1].parse().ok()?;
    let day: u32 = caps[2].parse().ok()?;
    let year = map_two_digit_year(caps[3].parse().ok()?);
    normalized_date(year, month, day)
}

fn create_from_format_dmy(date: &str) -> Option<DateTime<FixedOffset>> {
    let caps = mdy_re().captures(date)?;
    let day: u32 = caps[1].parse().ok()?;
    let month: u32 = caps[2].parse().ok()?;
    let year = map_two_digit_year(caps[3].parse().ok()?);
    normalized_date(year, month, day)
}

fn map_two_digit_year(y: u32) -> i32 {
    if y <= 69 {
        2000 + y as i32
    } else {
        1900 + y as i32
    }
}

fn normalized_date(year: i32, month: u32, day: u32) -> Option<DateTime<FixedOffset>> {
    let mut base = NaiveDate::from_ymd_opt(year, 1, 1)?;
    base = if month >= 1 {
        base.checked_add_months(Months::new(month - 1))?
    } else {
        base.checked_sub_months(Months::new(1))?
    };
    base = if day >= 1 {
        base.checked_add_days(Days::new((day - 1) as u64))?
    } else {
        base.checked_sub_days(Days::new(1))?
    };
    Some(DateTime::from_naive_utc_and_offset(
        base.and_hms_opt(0, 0, 0)?,
        FixedOffset::east_opt(0)?,
    ))
}

/// The subset of PHP's `strtotime`/date parser that MAL pages need.
fn parse_php_datetime(s: &str) -> Option<DateTime<FixedOffset>> {
    let now = Utc::now().fixed_offset();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = whitespace_re().replace_all(trimmed, " ").to_string();
    // PHP's date parser ignores trailing periods ("May 1, 1979." parses fine).
    let normalized = normalized.trim_end_matches('.').trim_end().to_string();
    let lower = normalized.to_ascii_lowercase();

    match lower.as_str() {
        "now" => return Some(now),
        "today" => return Some(start_of_day(now)),
        "tomorrow" => return Some(start_of_day(now + Duration::days(1))),
        "yesterday" => return Some(start_of_day(now - Duration::days(1))),
        _ => {}
    }

    // "9 hours ago", "45 minutes ago", "3 weeks ago", "2 months ago", ...
    if let Some(caps) = relative_ago_re().captures(&lower) {
        let n: i64 = caps[1].parse().ok()?;
        return relative_sub(now, n, &caps[2]);
    }

    // "Today, 1:05 PM" / "Yesterday, 12:04 PM" / "Tomorrow, 9:00 AM"
    if let Some(caps) = day_time_re().captures(&lower) {
        let base = match &caps[1] {
            "today" => now,
            "yesterday" => now - Duration::days(1),
            "tomorrow" => now + Duration::days(1),
            _ => return None,
        };
        let time = parse_clock(
            &caps[2],
            caps.get(3).map(|m| m.as_str()),
            caps.get(4).map(|m| m.as_str()),
            caps.get(5).map(|m| m.as_str()),
        )?;
        return Some(DateTime::from_naive_utc_and_offset(
            base.date_naive().and_time(time),
            *base.offset(),
        ));
    }

    // "Jul 4, 2021 9:22 PM", "May 25, 8:15 PM", "Oct 3, 2021 11:59 AM",
    // "Mar 26, 09:45 PM", "Jul 4, 2021"
    if let Some(caps) = month_day_re().captures(&lower) {
        let month = month_number(&caps[1])?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = match caps.get(3) {
            Some(y) => y.as_str().parse().ok()?,
            None => now.year(),
        };
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        let time = match caps.get(4) {
            Some(h) => parse_clock(
                h.as_str(),
                caps.get(5).map(|m| m.as_str()),
                caps.get(6).map(|m| m.as_str()),
                caps.get(7).map(|m| m.as_str()),
            )?,
            None => chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
        };
        return Some(DateTime::from_naive_utc_and_offset(
            date.and_time(time),
            FixedOffset::east_opt(0)?,
        ));
    }

    // "4 Jul 2021", "4 July 2021"
    if let Some(caps) = day_month_re().captures(&lower) {
        let day: u32 = caps[1].parse().ok()?;
        let month = month_number(&caps[2])?;
        let year: i32 = caps[3].parse().ok()?;
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        return Some(DateTime::from_naive_utc_and_offset(
            date.and_hms_opt(0, 0, 0)?,
            FixedOffset::east_opt(0)?,
        ));
    }

    // "Apr 2020" / "April 2020" (no day -> first of month)
    if let Some(caps) = month_year_re().captures(&lower) {
        let month = month_number(&caps[1])?;
        let year: i32 = caps[2].parse().ok()?;
        return utc_date(year, month, 1);
    }

    // "7/4/2021" (American m/d/Y), 2-digit years map like PHP
    if let Some(caps) = slash_mdy_re().captures(&lower) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year_digits = caps[3].len();
        let raw_year: u32 = caps[3].parse().ok()?;
        let year = if year_digits <= 2 {
            map_two_digit_year(raw_year)
        } else {
            raw_year as i32
        };
        return utc_date(year, month, day);
    }

    // "2021/07/04"
    if let Some(caps) = slash_ymd_re().captures(&lower) {
        let year: i32 = caps[1].parse().ok()?;
        let month: u32 = caps[2].parse().ok()?;
        let day: u32 = caps[3].parse().ok()?;
        return utc_date(year, month, day);
    }

    // "2021-07-04", "2021-07-04 21:22:00", "2021-07-04T21:22:00+02:00", ...
    parse_iso(&normalized)
}

fn parse_iso(s: &str) -> Option<DateTime<FixedOffset>> {
    let caps = iso_re().captures(s)?;
    let year: i32 = caps[1].parse().ok()?;
    let month: u32 = caps[2].parse().ok()?;
    let day: u32 = caps[3].parse().ok()?;
    let date = NaiveDate::from_ymd_opt(year, month, day)?;

    let time = match caps.get(4) {
        Some(h) => {
            let hour: u32 = h.as_str().parse().ok()?;
            let minute: u32 = caps[5].parse().ok()?;
            let second: u32 = caps
                .get(6)
                .map(|m| m.as_str().parse().ok())
                .unwrap_or(Some(0))?;
            chrono::NaiveTime::from_hms_opt(hour, minute, second)?
        }
        None => chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
    };

    let offset = match caps.get(7) {
        Some(z) => parse_offset(z.as_str())?,
        None => FixedOffset::east_opt(0)?,
    };
    // The parsed time is a wall clock; attach the offset without shifting it.
    offset.from_local_datetime(&date.and_time(time)).single()
}

fn parse_offset(z: &str) -> Option<FixedOffset> {
    if z.eq_ignore_ascii_case("z") {
        return FixedOffset::east_opt(0);
    }
    let sign = &z[..1];
    let rest = &z[1..];
    let (h, m) = if let Some((h, m)) = rest.split_once(':') {
        (h, m)
    } else if rest.len() == 4 {
        (&rest[..2], &rest[2..])
    } else {
        return None;
    };
    let hours: i32 = h.parse().ok()?;
    let minutes: i32 = m.parse().ok()?;
    let seconds = hours * 3600 + minutes * 60;
    match sign {
        "+" => FixedOffset::east_opt(seconds),
        "-" => FixedOffset::west_opt(seconds),
        _ => None,
    }
}

fn parse_clock(
    hour: &str,
    minute: Option<&str>,
    second: Option<&str>,
    meridiem: Option<&str>,
) -> Option<chrono::NaiveTime> {
    let mut hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute?.parse().ok()?;
    let second: u32 = second.map(|s| s.parse().ok()).unwrap_or(Some(0))?;
    if let Some(m) = meridiem {
        let pm = m.eq_ignore_ascii_case("pm");
        let am = m.eq_ignore_ascii_case("am");
        if !pm && !am {
            return None;
        }
        if hour == 12 {
            hour = 0;
        }
        if pm {
            hour += 12;
        }
    }
    chrono::NaiveTime::from_hms_opt(hour, minute, second)
}

/// "2 months ago" (PHP uses calendar-aware modify for months/years).
fn relative_sub(now: DateTime<FixedOffset>, n: i64, unit: &str) -> Option<DateTime<FixedOffset>> {
    let lower = unit.to_ascii_lowercase();
    match lower.as_str() {
        "sec" | "secs" | "second" | "seconds" => Some(now - Duration::seconds(n)),
        "min" | "mins" | "minute" | "minutes" => Some(now - Duration::minutes(n)),
        "hour" | "hours" => Some(now - Duration::hours(n)),
        "day" | "days" => Some(now - Duration::days(n)),
        "week" | "weeks" => Some(now - Duration::weeks(n)),
        "month" | "months" => now.checked_sub_months(Months::new(n as u32)),
        "year" | "years" => now.checked_sub_months(Months::new((n * 12) as u32)),
        _ => None,
    }
}

fn start_of_day(now: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    DateTime::from_naive_utc_and_offset(
        now.date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is always valid"),
        *now.offset(),
    )
}

/// Wall-clock parse used by `parse_date_time_pst` (no offset in the string).
fn parse_naive_datetime(s: &str) -> Option<NaiveDateTime> {
    let trimmed = s.trim();
    let normalized = whitespace_re().replace_all(trimmed, " ").to_string();

    // Strings that carry an explicit offset keep it in PHP as well.
    if let Some(dt) = parse_iso(&normalized) {
        return Some(dt.naive_local());
    }

    let lower = normalized.to_ascii_lowercase();
    if let Some(caps) = month_day_re().captures(&lower) {
        let month = month_number(&caps[1])?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = match caps.get(3) {
            Some(y) => y.as_str().parse().ok()?,
            None => Utc::now().year(),
        };
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        let time = match caps.get(4) {
            Some(h) => parse_clock(
                h.as_str(),
                caps.get(5).map(|m| m.as_str()),
                caps.get(6).map(|m| m.as_str()),
                caps.get(7).map(|m| m.as_str()),
            )?,
            None => chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
        };
        return Some(date.and_time(time));
    }

    None
}

/// UTC offset in seconds for `America/Los_Angeles` at the given wall clock.
fn la_utc_offset_seconds(naive: NaiveDateTime) -> i32 {
    let year = naive.year();
    let (start, end) = if year >= 2007 {
        (
            nth_weekday_of_month(year, 3, Weekday::Sun, 2).and_hms_opt(2, 0, 0),
            nth_weekday_of_month(year, 11, Weekday::Sun, 1).and_hms_opt(2, 0, 0),
        )
    } else {
        (
            nth_weekday_of_month(year, 4, Weekday::Sun, 1).and_hms_opt(2, 0, 0),
            nth_weekday_of_month(year, 10, Weekday::Sun, 0).and_hms_opt(2, 0, 0),
        )
    };
    let in_dst = match (start, end) {
        (Some(start), Some(end)) => naive >= start && naive < end,
        _ => false,
    };
    if in_dst {
        -7 * 3600
    } else {
        -8 * 3600
    }
}

/// `n`-th (1-based) `wd` of `month`; `n == 0` means the last one.
fn nth_weekday_of_month(year: i32, month: u32, wd: Weekday, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    if n == 0 {
        let last = NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .expect("valid month")
        .pred_opt()
        .expect("valid date");
        let diff = (last.weekday().num_days_from_sunday() + 7 - wd.num_days_from_sunday()) % 7;
        return last - Duration::days(diff as i64);
    }
    let first_wd = first.weekday().num_days_from_sunday();
    let target = wd.num_days_from_sunday();
    let day = 1 + (target + 7 - first_wd) % 7 + (n - 1) * 7;
    NaiveDate::from_ymd_opt(year, month, day).expect("weekday exists within month")
}

/// PHP `createFromFormat('M')`: 3-letter abbreviation, `Sept`, or full name.
fn month_number(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn month_comma_year_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([0-9A-Za-z_]{3}), ([0-9]{4})").expect("valid regex"))
}

fn month_comma_year_anchored_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([A-Za-z]+), ([0-9]{4})$").expect("valid regex"))
}

fn readable_date_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z]+ [0-9]+, [0-9]{4}").expect("valid regex"))
}

fn four_digits_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[0-9]{4}").expect("valid regex"))
}

fn mdy_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([0-9]{1,2})-([0-9]{1,2})-([0-9]{1,2})$").expect("valid regex"))
}

fn whitespace_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("valid regex"))
}

fn relative_ago_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^([0-9]+) (sec|secs|second|seconds|min|mins|minute|minutes|hour|hours|day|days|week|weeks|month|months|year|years) ago$",
        )
        .expect("valid regex")
    })
}

fn day_time_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(today|yesterday|tomorrow),? ([0-9]{1,2}):([0-9]{2})(?::([0-9]{2}))? ?(am|pm)?$",
        )
        .expect("valid regex")
    })
}

fn month_day_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^([a-z]+) ([0-9]{1,2})(?:,? ([0-9]{4}))?(?:,? ([0-9]{1,2}):([0-9]{2})(?::([0-9]{2}))? ?(am|pm)?)?$",
        )
        .expect("valid regex")
    })
}

fn day_month_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([0-9]{1,2}) ([a-z]+) ([0-9]{4})$").expect("valid regex"))
}

fn month_year_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-z]+),? ([0-9]{4})$").expect("valid regex"))
}

fn slash_mdy_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([0-9]{1,2})/([0-9]{1,2})/([0-9]{2,4})$").expect("valid regex"))
}

fn slash_ymd_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([0-9]{4})/([0-9]{1,2})/([0-9]{1,2})$").expect("valid regex"))
}

fn iso_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})(?:[ tT]([0-9]{1,2}):([0-9]{2})(?::([0-9]{2})(?:\.[0-9]+)?)?(?: ?(z|Z|[+-][0-9]{2}:?[0-9]{2}))?)?$",
        )
        .expect("valid regex")
    })
}

fn duration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([0-9]{2}):([0-9]{2}):([0-9]{2})").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(dt: &DateTime<FixedOffset>, f: &str) -> String {
        dt.format(f).to_string()
    }

    // Ported from test/JikanTest/Helper/ParserTest.php
    #[test]
    fn it_gets_dates() {
        let date = parse_date("2011").expect("year");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2011-01-01");

        let date = parse_date("Dec 1, 2004").expect("Dec 1, 2004");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2004-12-01");

        let date = parse_date("Dec, 2004").expect("Dec, 2004");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2004-12-01");

        let date = parse_date("Jul 4, 2021 9:22 PM").expect("datetime");
        assert_eq!(fmt(&date, "%Y-%m-%d %H:%M"), "2021-07-04 21:22");

        let date = parse_date("May 25, 8:15 PM").expect("no year");
        assert_eq!(fmt(&date, "%m-%d %H:%M"), "05-25 20:15");

        let date = parse_date("Yesterday, 12:04 PM").expect("yesterday");
        assert_eq!(fmt(&date, "%H:%M"), "12:04");
        let now = Utc::now().fixed_offset();
        let diff_days = (now.date_naive() - date.date_naive()).num_days();
        assert_eq!(diff_days, 1);

        let before = Utc::now().fixed_offset();
        let date = parse_date("9 hours ago").expect("hours ago");
        let after = Utc::now().fixed_offset();
        let expected = before - Duration::hours(9);
        assert!((date - expected).num_seconds().abs() <= 1);
        assert!(date >= before - Duration::hours(9) - Duration::seconds(1));
        assert!(date <= after - Duration::hours(9) + Duration::seconds(1));

        let date = parse_date("45 minutes ago").expect("minutes ago");
        let expected = Utc::now().fixed_offset() - Duration::minutes(45);
        assert!((date - expected).num_seconds().abs() <= 2);

        assert!(parse_date("?").is_none());
    }

    #[test]
    fn parse_date_quirks() {
        // "Apr, 2020" goes through the (\w{3}), (\d{4}) branch
        let date = parse_date("Apr, 2020").expect("Apr, 2020");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2020-04-01");

        // "Apr 2020" goes through the regular parser
        let date = parse_date("Apr 2020").expect("Apr 2020");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2020-04-01");

        // "Dec 2004" no comma, still first of month
        let date = parse_date("Dec 2004").expect("Dec 2004");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2004-12-01");

        // leading zero hour
        let date = parse_date("Mar 26, 09:45 PM").expect("Mar 26");
        assert_eq!(fmt(&date, "%m-%d %H:%M"), "03-26 21:45");

        // ISO with offset is preserved
        let date = parse_date("2021-07-04T21:22:00+02:00").expect("offset");
        assert_eq!(date.offset().local_minus_utc(), 7200);

        let date = parse_date("2021-07-04 21:22:00").expect("space iso");
        assert_eq!(fmt(&date, "%Y-%m-%d %H:%M:%S"), "2021-07-04 21:22:00");

        // PHP ignores trailing periods
        let date = parse_date("May 1, 1979.").expect("trailing dot");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1979-05-01");

        // American slash dates with 2-digit years
        let date = parse_date("7/4/21").expect("slash 2-digit");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2021-07-04");
        let date = parse_date("7/4/70").expect("slash 70");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1970-07-04");
        assert!(parse_date("13/01/2021").is_none());
    }

    #[test]
    fn parse_forum_date_appends_year() {
        let date = parse_forum_date("Dec 1").expect("forum date");
        assert_eq!(fmt(&date, "%m-%d"), "12-01");
        // The appended year must be the current one.
        assert_eq!(date.year(), Utc::now().year());
    }

    #[test]
    fn parse_date_mdy_dmy() {
        let date = parse_date_mdy(Some("-"));
        assert!(date.is_none());
        assert!(parse_date_mdy(None).is_none());

        // ??-??-<current 2-digit year> -> null
        let current = current_two_digit_year();
        assert!(parse_date_mdy(Some(&format!("??-??-{current}"))).is_none());
        // but another year parses
        let other = if current == "26" { "25" } else { "26" };
        let date = parse_date_mdy(Some(&format!("??-??-{other}"))).expect("mdy");
        assert_eq!(fmt(&date, "%m-%d"), "01-01");

        let date = parse_date_mdy(Some("01-02-21")).expect("mdy");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2021-01-02");

        // 2-digit year mapping
        let date = parse_date_mdy(Some("01-02-70")).expect("mdy 70");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1970-01-02");
        let date = parse_date_mdy(Some("01-02-69")).expect("mdy 69");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2069-01-02");

        // overflow normalization
        let date = parse_date_mdy(Some("13-01-21")).expect("month 13");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2022-01-01");
        let date = parse_date_mdy(Some("02-30-21")).expect("day 30");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2021-03-02");
        let date = parse_date_mdy(Some("00-00-00")).expect("zeros");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1999-11-30");

        // dmy order
        let date = parse_date_dmy(Some("01-02-21")).expect("dmy");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "2021-02-01");
        let date = parse_date_dmy(Some("31-12-99")).expect("dmy 99");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1999-12-31");

        // invalid formats fail
        assert!(parse_date_mdy(Some("01-02-2021")).is_none());
        assert!(parse_date_mdy(Some("01-02-21x")).is_none());
    }

    #[test]
    fn parse_date_mdy_readable_cases() {
        let date = parse_date_mdy_readable("May, 1979").expect("May, 1979");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1979-05-01");

        let date = parse_date_mdy_readable("May 1, 1979").expect("May 1, 1979");
        assert_eq!(fmt(&date, "%Y-%m-%d"), "1979-05-01");

        assert!(parse_date_mdy_readable("1979").is_none());
        assert!(parse_date_mdy_readable("May,1979").is_none());
    }

    #[test]
    fn parse_date_time_pst_converts_to_utc() {
        // Expected values captured from PHP 8.5 `Parser::parseDateTimePST`.
        // Nov 2 2021 is still PDT (DST ends Nov 7, 2021): 10:00 -> 17:00 UTC
        let dt = parse_date_time_pst("Nov 2, 2021 10:00 AM").expect("pdt");
        assert_eq!(fmt(&dt, "%Y-%m-%d %H:%M"), "2021-11-02 17:00");
        assert_eq!(dt.offset().local_minus_utc(), 0);

        // Dec 2 2021 is PST (UTC-8): 10:00 -> 18:00 UTC
        let dt = parse_date_time_pst("Dec 2, 2021 10:00 AM").expect("pst");
        assert_eq!(fmt(&dt, "%Y-%m-%d %H:%M"), "2021-12-02 18:00");

        // Jun 1 2021 is PDT (UTC-7)
        let dt = parse_date_time_pst("Jun 1, 2021 10:00 AM").expect("pdt");
        assert_eq!(fmt(&dt, "%Y-%m-%d %H:%M"), "2021-06-01 17:00");

        assert!(parse_date_time_pst("?").is_none());
    }

    #[test]
    fn duration_seconds() {
        assert_eq!(parse_duration_to_seconds("00:24:00"), Some(1440));
        assert_eq!(parse_duration_to_seconds("01:02:03"), Some(3723));
        assert_eq!(parse_duration_to_seconds("24 min per ep"), None);
        assert_eq!(parse_duration_to_seconds(""), None);
        assert_eq!(parse_duration_to_seconds(" 00:24:00 "), Some(1440));
        assert_eq!(parse_duration_to_seconds("24:00"), None);
    }

    #[test]
    fn format_atom_is_php_date_atom() {
        let date = parse_date("2024-01-06").unwrap();
        assert_eq!(format_atom(&date), "2024-01-06T00:00:00+00:00");
        let date = parse_date("2021-07-04T21:22:00+02:00").unwrap();
        assert_eq!(format_atom(&date), "2021-07-04T21:22:00+02:00");
        let utc: DateTime<Utc> = Utc::now();
        assert_eq!(format_atom(&utc).len(), 25);
        assert!(format_atom(&utc).ends_with("+00:00"));
    }

    #[test]
    fn la_dst_offsets() {
        let standard = NaiveDate::from_ymd_opt(2021, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(la_utc_offset_seconds(standard), -8 * 3600);
        let daylight = NaiveDate::from_ymd_opt(2021, 7, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(la_utc_offset_seconds(daylight), -7 * 3600);
    }
}
