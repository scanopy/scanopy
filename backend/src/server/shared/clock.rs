use chrono::{DateTime, SubsecRound, Utc};

/// The current time at microsecond precision.
///
/// Postgres `timestamptz` stores microseconds, and `Utc::now()` returns nanoseconds on Linux. A
/// value taken from `Utc::now()` and written to the database no longer equals itself once read
/// back. Truncating here keeps an in-memory timestamp equal to its stored copy. clippy.toml bans
/// `Utc::now` everywhere else.
#[allow(clippy::disallowed_methods)]
pub fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(6)
}
