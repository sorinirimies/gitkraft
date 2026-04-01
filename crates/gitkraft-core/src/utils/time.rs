//! Time formatting and OID helpers.

/// Return a human-friendly relative-time string for the given UTC timestamp.
///
/// # Examples
///
/// - `"just now"`
/// - `"1 minute ago"` / `"5 minutes ago"`
/// - `"1 hour ago"` / `"3 hours ago"`
/// - `"1 day ago"` / `"2 days ago"`
/// - `"1 week ago"` / `"3 weeks ago"`
/// - `"1 month ago"` / `"6 months ago"`
/// - `"1 year ago"` / `"2 years ago"`
pub fn relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(dt);

    let seconds = duration.num_seconds();
    if seconds < 0 {
        return "in the future".to_string();
    }

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;
    let months = days / 30;
    let years = days / 365;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes == 1 {
        "1 minute ago".to_string()
    } else if minutes < 60 {
        format!("{minutes} minutes ago")
    } else if hours == 1 {
        "1 hour ago".to_string()
    } else if hours < 24 {
        format!("{hours} hours ago")
    } else if days == 1 {
        "1 day ago".to_string()
    } else if days < 7 {
        format!("{days} days ago")
    } else if weeks == 1 {
        "1 week ago".to_string()
    } else if weeks < 5 {
        format!("{weeks} weeks ago")
    } else if months == 1 {
        "1 month ago".to_string()
    } else if months < 12 {
        format!("{months} months ago")
    } else if years == 1 {
        "1 year ago".to_string()
    } else {
        format!("{years} years ago")
    }
}

/// Format a `git2::Oid` as a full hex string.
pub fn fmt_oid(oid: git2::Oid) -> String {
    oid.to_string()
}

/// Format a `git2::Oid` as an abbreviated hex string (first 7 characters).
pub fn short_oid(oid: git2::Oid) -> String {
    let full = oid.to_string();
    full[..7.min(full.len())].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn relative_time_just_now() {
        let now = Utc::now();
        assert_eq!(relative_time(now), "just now");
    }

    #[test]
    fn relative_time_one_minute() {
        let t = Utc::now() - chrono::Duration::minutes(1);
        assert_eq!(relative_time(t), "1 minute ago");
    }

    #[test]
    fn relative_time_minutes() {
        let t = Utc::now() - chrono::Duration::minutes(5);
        assert_eq!(relative_time(t), "5 minutes ago");
    }

    #[test]
    fn relative_time_one_hour() {
        let t = Utc::now() - chrono::Duration::hours(1);
        assert_eq!(relative_time(t), "1 hour ago");
    }

    #[test]
    fn relative_time_hours() {
        let t = Utc::now() - chrono::Duration::hours(3);
        assert_eq!(relative_time(t), "3 hours ago");
    }

    #[test]
    fn relative_time_one_day() {
        let t = Utc::now() - chrono::Duration::days(1);
        assert_eq!(relative_time(t), "1 day ago");
    }

    #[test]
    fn relative_time_days() {
        let t = Utc::now() - chrono::Duration::days(4);
        assert_eq!(relative_time(t), "4 days ago");
    }

    #[test]
    fn relative_time_one_week() {
        let t = Utc::now() - chrono::Duration::weeks(1);
        assert_eq!(relative_time(t), "1 week ago");
    }

    #[test]
    fn relative_time_weeks() {
        let t = Utc::now() - chrono::Duration::weeks(3);
        assert_eq!(relative_time(t), "3 weeks ago");
    }

    #[test]
    fn relative_time_one_month() {
        let t = Utc::now() - chrono::Duration::days(35);
        assert_eq!(relative_time(t), "1 month ago");
    }

    #[test]
    fn relative_time_months() {
        let t = Utc::now() - chrono::Duration::days(180);
        assert_eq!(relative_time(t), "6 months ago");
    }

    #[test]
    fn relative_time_one_year() {
        let t = Utc::now() - chrono::Duration::days(400);
        assert_eq!(relative_time(t), "1 year ago");
    }

    #[test]
    fn relative_time_years() {
        let t = Utc::now() - chrono::Duration::days(800);
        assert_eq!(relative_time(t), "2 years ago");
    }

    #[test]
    fn relative_time_future() {
        let t = Utc::now() + chrono::Duration::hours(1);
        assert_eq!(relative_time(t), "in the future");
    }

    #[test]
    fn fmt_oid_full() {
        let oid = git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap();
        assert_eq!(fmt_oid(oid), "abcdef1234567890abcdef1234567890abcdef12");
    }

    #[test]
    fn short_oid_seven_chars() {
        let oid = git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap();
        assert_eq!(short_oid(oid), "abcdef1");
    }
}
