// AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
// Run `cargo test --test integration generate_fixtures` to regenerate
//
// Sources:
// - Enterprise Numbers: https://www.iana.org/assignments/enterprise-numbers/enterprise-numbers
// - IANAifType: https://www.iana.org/assignments/ianaiftype-mib/ianaiftype-mib

mod enterprise_numbers;
mod if_types;

pub use enterprise_numbers::{extract_enterprise_number, get_enterprise_name};
pub use if_types::get_if_type_name;
