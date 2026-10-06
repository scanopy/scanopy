//! Custom field formatter for event logs.
//!
//! Renders each event-log line as a color-coded `<label>: ` prefix followed by
//! the JSON payload, e.g. `subnet_created: {"id":...}`. Regular log lines (no
//! `log_label` field) are rendered normally — message first, then any
//! remaining fields as `key=value`.
//!
//! Color is real ANSI written straight to the output (not the default field
//! path, which escapes control characters), gated by the fmt layer's own ANSI
//! setting via [`Writer::has_ansi_escapes`]. That setting is driven once by
//! [`supports_ansi`] on `.with_ansi(..)`, so tracing's built-in level/timestamp
//! coloring and our label share a single decision.

use std::fmt::{self, Write as _};
use std::io::IsTerminal;

use tracing::field::{Field, Visit};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::format::Writer;

/// Whether stdout should receive ANSI color. `is_terminal` alone is not enough
/// — editor task-runners and multiplexers report a tty but don't render SGR —
/// so we also require the terminal to advertise color support, and honor
/// `NO_COLOR` / `CLICOLOR_FORCE`.
pub fn supports_ansi() -> bool {
    if anstyle_query::no_color() {
        false
    } else if anstyle_query::clicolor_force() {
        true
    } else {
        std::io::stdout().is_terminal() && anstyle_query::term_supports_ansi_color()
    }
}

/// Field formatter that pulls `log_label` / `log_color` out of an event and
/// renders a colored label prefix.
pub struct LabelFields;

#[derive(Default)]
struct LabelVisitor {
    label: Option<String>,
    color: Option<String>,
    message: Option<String>,
    rest: String,
}

impl Visit for LabelVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "log_label" => self.label = Some(value.to_owned()),
            "log_color" => self.color = Some(value.to_owned()),
            _ => self.record_debug(field, &value),
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match field.name() {
            "message" => self.message = Some(format!("{value:?}")),
            "log_label" | "log_color" => {}
            name => {
                let _ = write!(self.rest, " {name}={value:?}");
            }
        }
    }
}

impl<'writer> FormatFields<'writer> for LabelFields {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visitor = LabelVisitor::default();
        fields.record(&mut visitor);

        if let Some(label) = &visitor.label {
            if writer.has_ansi_escapes() {
                let code = visitor.color.as_deref().unwrap_or("100;97");
                write!(writer, "\x1b[{code}m {label} \x1b[0m ")?;
            } else {
                write!(writer, "{label}: ")?;
            }
        }
        if let Some(message) = &visitor.message {
            write!(writer, "{message}")?;
        }
        write!(writer, "{}", visitor.rest)
    }
}
