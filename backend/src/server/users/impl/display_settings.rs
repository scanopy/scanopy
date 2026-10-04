use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Per-user display preferences for dates and times in the UI.
///
/// Stored as a JSONB blob with `#[serde(default)]` on the struct, so a stored
/// object missing a key (including the `{}` every existing row starts with)
/// reads as that field's default. Every default reproduces the UI's
/// behaviour from before the setting existed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(default)]
pub struct DisplaySettings {
    /// Order of day, month and year in displayed dates.
    pub date_order: DateOrder,
    /// 12- or 24-hour clock for displayed times.
    pub clock: ClockFormat,
    /// IANA time zone dates are shown in. `None` uses the browser's zone.
    pub time_zone: Option<String>,
    /// First day of the week in day pickers.
    pub week_start: WeekStart,
    /// Whether recent events show as relative ("3h ago") or absolute times.
    pub timestamps: TimestampStyle,
    /// Cell padding and row height in entity tables.
    pub table_density: TableDensity,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DateOrder {
    /// The browser locale's own order and style.
    #[default]
    BrowserDefault,
    /// ISO 8601, `2026-10-08`.
    Iso,
    /// `8 Oct 2026`.
    DayFirst,
    /// `Oct 8, 2026`.
    MonthFirst,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClockFormat {
    /// The browser locale's own clock.
    #[default]
    BrowserDefault,
    TwelveHour,
    TwentyFourHour,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimestampStyle {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TableDensity {
    #[default]
    Comfortable,
    /// Tighter cell padding, shorter rows and smaller row actions.
    Compact,
}

impl DisplaySettings {
    /// Whether `time_zone` is unset or names a zone in the IANA database.
    pub fn has_valid_time_zone(&self) -> bool {
        self.time_zone
            .as_deref()
            .is_none_or(|tz| tz.parse::<chrono_tz::Tz>().is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_keys_read_as_defaults() {
        let empty: DisplaySettings = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, DisplaySettings::default());

        let partial: DisplaySettings = serde_json::from_str(r#"{"date_order":"iso"}"#).unwrap();
        assert_eq!(
            partial,
            DisplaySettings {
                date_order: DateOrder::Iso,
                ..DisplaySettings::default()
            }
        );
    }

    #[test]
    fn table_density_survives_storage_round_trip() {
        // The JSONB column is bound with `to_value` and read back with `from_value`.
        let settings = DisplaySettings {
            table_density: TableDensity::Compact,
            ..DisplaySettings::default()
        };
        let stored = serde_json::to_value(&settings).unwrap();
        let read: DisplaySettings = serde_json::from_value(stored).unwrap();
        assert_eq!(read.table_density, TableDensity::Compact);

        let before_density: DisplaySettings =
            serde_json::from_str(r#"{"clock":"twelve_hour"}"#).unwrap();
        assert_eq!(before_density.table_density, TableDensity::Comfortable);
    }

    #[test]
    fn time_zone_must_be_iana_or_unset() {
        let with = |tz: Option<&str>| DisplaySettings {
            time_zone: tz.map(str::to_string),
            ..DisplaySettings::default()
        };
        assert!(with(None).has_valid_time_zone());
        assert!(with(Some("Europe/Berlin")).has_valid_time_zone());
        assert!(!with(Some("Mars/Olympus_Mons")).has_valid_time_zone());
    }
}
