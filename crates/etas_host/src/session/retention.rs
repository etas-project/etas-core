use std::time::{SystemTime, UNIX_EPOCH};

use crate::{HostError, HostErrorCode, RetentionPolicy, SessionMessage};

const SECONDS_PER_DAY: i128 = 86_400;

pub(super) fn retain_messages(
    messages: &[SessionMessage],
    policy: &RetentionPolicy,
) -> Result<Vec<SessionMessage>, HostError> {
    match policy {
        RetentionPolicy::Forever => Ok(messages.to_vec()),
        RetentionPolicy::Days(days) => {
            let now = current_unix_seconds()?;
            let retained_after =
                now.saturating_sub(i128::from(*days).saturating_mul(SECONDS_PER_DAY));
            messages
                .iter()
                .filter_map(|message| match session_message_timestamp(message) {
                    Ok(created_at) if created_at >= retained_after => Some(Ok(message.clone())),
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect()
        }
    }
}

fn session_message_timestamp(message: &SessionMessage) -> Result<i128, HostError> {
    parse_session_timestamp(&message.created_at).ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "session message timestamp is not valid for retention",
        )
        .with_detail("session", message.session.id.clone())
        .with_detail("message", message.id.clone())
        .with_detail("created_at", message.created_at.clone())
    })
}

fn current_unix_seconds() -> Result<i128, HostError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i128::from(duration.as_secs()))
        .map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "system clock is before UNIX epoch",
            )
            .with_detail("error", error.to_string())
        })
}

fn parse_session_timestamp(value: &str) -> Option<i128> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return value.parse::<i128>().ok();
    }
    parse_rfc3339_timestamp(value)
}

fn parse_rfc3339_timestamp(value: &str) -> Option<i128> {
    let (timestamp, offset_seconds) = if let Some(timestamp) = value.strip_suffix('Z') {
        (timestamp, 0)
    } else {
        let offset = value.get(value.len().checked_sub(6)?..)?;
        let sign = match offset.as_bytes().first().copied()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        if offset.as_bytes().get(3).copied()? != b':' {
            return None;
        }
        let hours = parse_fixed_u32(offset.get(1..3)?)?;
        let minutes = parse_fixed_u32(offset.get(4..6)?)?;
        if hours > 23 || minutes > 59 {
            return None;
        }
        (
            &value[..value.len() - 6],
            sign * ((hours * 3_600 + minutes * 60) as i128),
        )
    };
    let (date, time) = timestamp.split_once('T')?;
    let (year, month, day) = parse_date(date)?;
    let (hour, minute, second) = parse_time(time)?;
    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400 + i128::from(hour * 3_600 + minute * 60 + second) - offset_seconds)
}

fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return None;
    }
    let year = value.get(0..4)?.parse::<i32>().ok()?;
    let month = parse_fixed_u32(value.get(5..7)?)?;
    let day = parse_fixed_u32(value.get(8..10)?)?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn parse_time(value: &str) -> Option<(u32, u32, u32)> {
    let value = value.split_once('.').map_or(value, |(seconds, _)| seconds);
    if value.len() != 8
        || value.as_bytes().get(2) != Some(&b':')
        || value.as_bytes().get(5) != Some(&b':')
    {
        return None;
    }
    let hour = parse_fixed_u32(value.get(0..2)?)?;
    let minute = parse_fixed_u32(value.get(3..5)?)?;
    let second = parse_fixed_u32(value.get(6..8)?)?;
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some((hour, minute, second.min(59)))
}

fn parse_fixed_u32(value: &str) -> Option<u32> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i128> {
    let year = i128::from(year) - i128::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i128::from(month);
    let day = i128::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}
