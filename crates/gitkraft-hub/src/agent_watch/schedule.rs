//! When a job is due: `daily 08:00`, `mon-fri 07:30`, `weekly`, `sat,sun 10:00`, `every 6h`, `every 30m`.

use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime};

/// A parsed schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schedule {
    /// On these weekdays (0 = Sunday) at this time of day.
    At {
        /// Weekdays, 0 = Sunday … 6 = Saturday.
        days: Vec<u8>,
        /// Time of day.
        time: NaiveTime,
    },
    /// Every so many minutes after the last run.
    Every(u32),
}

const NAMES: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

fn day_of(word: &str) -> Option<u8> {
    let w = word.trim().to_ascii_lowercase();
    let w = w.get(..3)?;
    NAMES.iter().position(|n| *n == w).map(|i| i as u8)
}

fn days_of(spec: &str) -> Result<Vec<u8>, String> {
    let spec = spec.trim().to_ascii_lowercase();
    let mut out: Vec<u8> = match spec.as_str() {
        "daily" | "everyday" | "*" => (0..7).collect(),
        "weekdays" => (1..6).collect(),
        "weekends" => vec![0, 6],
        "weekly" => vec![0],
        _ => {
            let mut v = Vec::new();
            for part in spec.split(',') {
                if let Some((a, b)) = part.split_once('-') {
                    let (a, b) = day_of(a)
                        .zip(day_of(b))
                        .ok_or_else(|| format!("unknown day range `{part}`"))?;
                    let mut d = a;
                    loop {
                        v.push(d);
                        if d == b {
                            break;
                        }
                        d = (d + 1) % 7;
                    }
                } else {
                    v.push(day_of(part).ok_or_else(|| format!("unknown day `{part}`"))?);
                }
            }
            v
        }
    };
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

impl Schedule {
    /// Parses a schedule.
    ///
    /// # Errors
    /// Says what is wrong with `text`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("every ") {
            let rest = rest.trim();
            let (n, unit) = rest.split_at(
                rest.find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(rest.len()),
            );
            let n: u32 = n
                .parse()
                .map_err(|_| format!("`{rest}`: use `every 30m` or `every 6h`"))?;
            let minutes = match unit.trim() {
                "m" | "min" => n,
                "h" | "hr" => n.saturating_mul(60),
                "d" => n.saturating_mul(1440),
                _ => return Err(format!("`{rest}`: use `every 30m` or `every 6h`")),
            };
            if minutes < 5 {
                return Err("every at least 5m".into());
            }
            return Ok(Self::Every(minutes));
        }
        let (days, time) = match text.rsplit_once(' ') {
            Some((d, t)) if t.contains(':') => (d, t),
            _ => (text, "08:00"),
        };
        let time = NaiveTime::parse_from_str(time, "%H:%M")
            .map_err(|_| format!("`{time}` is not a time: use HH:MM"))?;
        Ok(Self::At {
            days: days_of(days)?,
            time,
        })
    }

    /// Back to text.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Every(m) if m % 60 == 0 => format!("every {}h", m / 60),
            Self::Every(m) => format!("every {m}m"),
            Self::At { days, time } => {
                let d = if days.len() == 7 {
                    "daily".to_string()
                } else if days == &[1, 2, 3, 4, 5] {
                    "weekdays".to_string()
                } else {
                    days.iter()
                        .map(|d| NAMES[*d as usize % 7])
                        .collect::<Vec<_>>()
                        .join(",")
                };
                format!("{d} {}", time.format("%H:%M"))
            }
        }
    }

    /// Is a run owed at `now` (local time), given when the job last ran? A slot missed while the
    /// machine was off is owed as soon as it is back, once.
    #[must_use]
    pub fn due(&self, now: NaiveDateTime, last: Option<NaiveDateTime>) -> bool {
        match self {
            Self::Every(m) => last.is_none_or(|l| now - l >= Duration::minutes(i64::from(*m))),
            Self::At { .. } => self
                .latest_slot(now)
                .is_some_and(|slot| last.is_none_or(|l| l < slot)),
        }
    }

    /// The most recent scheduled moment at or before `now` (within the last week).
    #[must_use]
    pub fn latest_slot(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        let Self::At { days, time } = self else {
            return None;
        };
        (0..=7).find_map(|back| {
            let date = now.date() - Duration::days(back);
            let slot = date.and_time(*time);
            let day = date.weekday().num_days_from_sunday() as u8;
            (days.contains(&day) && slot <= now).then_some(slot)
        })
    }

    /// The next moment after `now`, for display.
    #[must_use]
    pub fn next(&self, now: NaiveDateTime, last: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
        match self {
            Self::Every(m) => {
                Some(last.map_or(now, |l| (l + Duration::minutes(i64::from(*m))).max(now)))
            }
            Self::At { days, time } => (0..=7).find_map(|ahead| {
                let date = now.date() + Duration::days(ahead);
                let slot = date.and_time(*time);
                let day = date.weekday().num_days_from_sunday() as u8;
                (days.contains(&day) && slot > now).then_some(slot)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").expect("time")
    }

    #[test]
    fn schedules_parse_and_print_back() {
        for (text, back) in [
            ("daily 08:00", "daily 08:00"),
            ("mon-fri 07:30", "weekdays 07:30"),
            ("weekdays 07:30", "weekdays 07:30"),
            ("sat,sun 10:00", "sun,sat 10:00"),
            ("weekly", "sun 08:00"),
            ("every 6h", "every 6h"),
            ("every 90m", "every 90m"),
            ("fri-mon 09:00", "sun,mon,fri,sat 09:00"),
        ] {
            assert_eq!(
                Schedule::parse(text).expect(text).describe(),
                back,
                "{text}"
            );
        }
        assert!(Schedule::parse("noday 08:00").is_err());
        assert!(Schedule::parse("daily 25:00").is_err());
        assert!(Schedule::parse("every 1m").is_err());
        assert!(Schedule::parse("every soon").is_err());
    }

    #[test]
    fn a_daily_job_is_due_once_after_its_time_and_catches_up_after_sleep() {
        let s = Schedule::parse("daily 08:00").expect("daily");
        let last = Some(at("2026-03-02 08:01"));
        assert!(!s.due(at("2026-03-02 23:00"), last), "already ran today");
        assert!(!s.due(at("2026-03-03 07:59"), last), "not yet");
        assert!(s.due(at("2026-03-03 08:00"), last));
        assert!(
            s.due(at("2026-03-03 15:00"), last),
            "the machine slept through 08:00"
        );
        assert!(s.due(at("2026-03-03 09:00"), None), "never ran");
    }

    #[test]
    fn weekday_jobs_skip_the_weekend() {
        // 2026-03-06 is a Friday, 03-07 Saturday, 03-09 Monday.
        let s = Schedule::parse("mon-fri 08:00").expect("weekdays");
        let friday_run = Some(at("2026-03-06 08:05"));
        assert!(!s.due(at("2026-03-07 12:00"), friday_run));
        assert!(!s.due(at("2026-03-08 12:00"), friday_run));
        assert!(!s.due(at("2026-03-09 07:00"), friday_run));
        assert!(s.due(at("2026-03-09 08:00"), friday_run));
        assert_eq!(
            s.next(at("2026-03-06 09:00"), friday_run),
            Some(at("2026-03-09 08:00"))
        );
    }

    #[test]
    fn an_interval_counts_from_the_last_run() {
        let s = Schedule::parse("every 6h").expect("interval");
        let last = Some(at("2026-03-02 08:00"));
        assert!(!s.due(at("2026-03-02 13:59"), last));
        assert!(s.due(at("2026-03-02 14:00"), last));
        assert!(s.due(at("2026-03-02 14:00"), None));
    }
}
