//! Display metadata for [`MatchConfidence`], emitted as `match-confidences.json` so the UI labels
//! and colours a confidence, and offers the services filter's options, from one source.
use crate::server::services::r#impl::patterns::MatchConfidence;
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// The serde name, which is also what `source.details.confidence` holds and what the
/// `match_confidences` filter binds.
impl HasId for MatchConfidence {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for MatchConfidence {
    fn color(&self) -> Color {
        match self {
            Self::NotApplicable => Color::Gray,
            Self::Low => Color::Red,
            Self::Medium => Color::Yellow,
            Self::High | Self::Certain => Color::Green,
        }
    }

    fn icon(&self) -> Icon {
        Icon::Gauge
    }
}

impl TypeMetadataProvider for MatchConfidence {
    fn name(&self) -> &'static str {
        match self {
            Self::NotApplicable => "Not Applicable",
            Self::Low => "Low Confidence",
            Self::Medium => "Medium Confidence",
            Self::High => "High Confidence",
            Self::Certain => "Certain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// The id is bound against the stored JSON, so it has to be exactly what serde writes.
    #[test]
    fn id_is_the_serde_name() {
        for confidence in MatchConfidence::iter() {
            assert_eq!(
                serde_json::to_value(confidence).unwrap(),
                serde_json::Value::String(confidence.id().to_string()),
            );
        }
    }
}
