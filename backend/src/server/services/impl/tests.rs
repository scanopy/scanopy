use strum::{IntoDiscriminant, IntoEnumIterator};

use crate::server::{
    ports::r#impl::base::PortType,
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{definitions::ServiceDefinition, patterns::Pattern},
    },
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::PathBuf,
};

#[test]
fn test_all_service_definitions_register() {
    use std::collections::HashMap;
    use std::fs;

    // Get all registered services using inventory
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    // Verify at least some services are registered
    assert!(
        !registry.is_empty(),
        "No service definitions registered! Check inventory setup."
    );

    // Verify no duplicate names
    let names: HashSet<_> = registry.iter().map(|s| s.name()).collect();
    assert_eq!(
        names.len(),
        registry.len(),
        "Duplicate service definition names found!"
    );

    // Get all declared modules from mod.rs
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let definitions_dir = manifest_dir.join("src/server/services/definitions");
    let mod_rs_path = definitions_dir.join("mod.rs");
    let mod_rs_content = fs::read_to_string(&mod_rs_path).expect("Failed to read mod.rs");

    let declared_modules: HashSet<String> = mod_rs_content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("pub mod ") && trimmed.ends_with(';') {
                let module_name = trimmed
                    .trim_start_matches("pub mod ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                Some(module_name)
            } else {
                None
            }
        })
        .collect();

    // Build map of declared module -> extracted service name
    let mut declared_services: HashMap<String, String> = HashMap::new();

    for module in &declared_modules {
        let file_path = definitions_dir.join(format!("{}.rs", module));

        if let Ok(content) = fs::read_to_string(&file_path)
            && let Some(service_name) = extract_service_name(&content)
        {
            declared_services.insert(module.clone(), service_name);
        }
    }

    // Check that all declared services are registered
    let registered_names: HashSet<String> = registry.iter().map(|s| s.name().to_string()).collect();

    let mut not_registered = Vec::new();
    for (module, service_name) in &declared_services {
        if !registered_names.contains(service_name) {
            not_registered.push((module.clone(), service_name.clone()));
        }
    }

    if !not_registered.is_empty() {
        panic!(
            "Service definitions are declared in mod.rs but NOT registered with inventory::submit!:\n{}\n\n\
            Each service definition file must include:\n\
            inventory::submit!(ServiceDefinitionFactory::new(create_service::<YourServiceStruct>));",
            not_registered
                .iter()
                .map(|(module, name)| format!("  - {} (service name: '{}')", module, name))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn test_all_service_definition_files_can_be_parsed() {
    use std::fs;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let definitions_dir = manifest_dir.join("src/server/services/definitions");

    // Get all .rs files (excluding mod.rs and example.rs)
    let rs_files: Vec<String> = fs::read_dir(&definitions_dir)
        .expect("Failed to read definitions directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let stem = path.file_stem()?.to_string_lossy().to_string();

            if path.extension()? == "rs" && stem != "mod" && stem != "example" {
                Some(stem)
            } else {
                None
            }
        })
        .collect();

    // Try to extract service name from each file
    let mut parse_failures = Vec::new();
    let mut empty_files = Vec::new();

    for filename in &rs_files {
        let file_path = definitions_dir.join(format!("{}.rs", filename));

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                if content.trim().is_empty() {
                    empty_files.push(filename.clone());
                } else if extract_service_name(&content).is_none() {
                    parse_failures.push(filename.clone());
                }
            }
            Err(e) => {
                panic!("Failed to read {}.rs: {}", filename, e);
            }
        }
    }

    let mut errors = Vec::new();

    if !empty_files.is_empty() {
        errors.push(format!(
            "Empty service definition files found:\n{}",
            empty_files
                .iter()
                .map(|f| format!("  - {}.rs", f))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    if !parse_failures.is_empty() {
        errors.push(format!(
            "Service definition files without parseable fn name(&self) method:\n{}\n\n\
            Each service definition must implement:\n\
            fn name(&self) -> &'static str {{\n\
                \"Service Name\"\n\
            }}",
            parse_failures
                .iter()
                .map(|f| format!("  - {}.rs", f))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    if !errors.is_empty() {
        panic!("{}", errors.join("\n\n"));
    }
}

#[test]
fn test_service_definition_has_required_fields() {
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    for service in registry {
        // Every service must have non-empty name
        assert!(
            !ServiceDefinition::name(&service).is_empty(),
            "Service has empty name"
        );

        // Name should be reasonable length (< 40 chars)
        assert!(
            service.name().len() < 40,
            "Service name '{}' is too long; must be < 40 characters",
            service.name()
        );

        // Every service must have description
        assert!(
            !service.description().is_empty(),
            "Service '{}' has empty description",
            service.name()
        );

        // Description should be reasonable length
        assert!(
            service.description().len() < 100,
            "Service '{}' description is too long; must be < 100 characters",
            service.name()
        );
    }
}

#[test]
fn test_no_duplicate_discovery_patterns() {
    let registry: Vec<_> = ServiceDefinitionRegistry::all_service_definitions()
        .into_iter()
        .filter(|s| !matches!(s.discovery_pattern(), Pattern::None))
        .collect();

    let mut duplicates: Vec<String> = Vec::new();

    for (i, service_a) in registry.iter().enumerate() {
        let pattern_a = service_a.discovery_pattern();

        for service_b in registry.iter().skip(i + 1) {
            let pattern_b = service_b.discovery_pattern();

            if pattern_a == pattern_b {
                duplicates.push(format!(
                    "  '{}' and '{}' share pattern: {}",
                    service_a.name(),
                    service_b.name(),
                    pattern_a
                ));
            }
        }
    }

    if !duplicates.is_empty() {
        panic!(
            "Duplicate discovery patterns found! Multiple services cannot share the same pattern:\n\n{}\n\n\
            Each service must have a unique discovery pattern to avoid ambiguous matches.\n\
            Consider:\n\
            1. Removing one of the duplicate service definitions\n\
            2. Adding additional criteria (AllOf with extra port, endpoint path, etc.)\n\
            3. Using a more specific endpoint match string",
            duplicates.join("\n")
        );
    }
}

#[test]
fn test_all_protocol_ports_have_generic_service() {
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    use crate::server::services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{definitions::ServiceDefinition, patterns::Pattern},
    };

    // Ports to skip - discovered via other mechanisms or require multi-signal matching
    let skip_ports: HashSet<PortType> =
        HashSet::from([PortType::Docker, PortType::DockerTls, PortType::Kubernetes]);

    // Get all well-known ports (non-Custom, non-Http*)
    let well_known_ports: Vec<PortType> = PortType::iter()
        .filter(|port| {
            if matches!(port, PortType::Custom(_)) {
                return false;
            }
            if skip_ports.contains(port) {
                return false;
            }
            let name = format!("{:?}", port);
            if name.starts_with("Http") || name.starts_with("Https") {
                return false;
            }
            true
        })
        .collect();

    let generic_services: Vec<_> = ServiceDefinitionRegistry::all_service_definitions()
        .into_iter()
        .filter(|s| s.is_generic())
        .collect();

    fn pattern_matches_port_alone(pattern: &Pattern, target_port: &PortType) -> bool {
        match pattern {
            Pattern::Port(port) => port == target_port,
            Pattern::Endpoint(port, _, _, _) => port == target_port,
            // ClientResponse integrations (SNMP, Docker) handle their own port detection
            // via credentialed probes — they don't need Pattern::Port coverage.
            Pattern::ClientResponse(_) => true,
            // `AllOf` counts for the same reason `AnyOf` does: `Pattern::ports()` flattens both,
            // so either shape puts the port into the light scan. The industrial protocols are
            // `AllOf([Port(p), ClientResponse(c)])` — the port is scanned, and the service is
            // claimed only once the probe has answered on it.
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => patterns
                .iter()
                .any(|p| pattern_matches_port_alone(p, target_port)),
            Pattern::Not(_) => false,
            _ => false,
        }
    }

    let mut uncovered_ports: Vec<PortType> = Vec::new();

    for port in &well_known_ports {
        let has_coverage = generic_services
            .iter()
            .any(|service| pattern_matches_port_alone(&service.discovery_pattern(), port));

        if !has_coverage {
            uncovered_ports.push(*port);
        }
    }

    if !uncovered_ports.is_empty() {
        let port_list: Vec<String> = uncovered_ports
            .iter()
            .map(|p| format!("  - {:?} ({})", p, p))
            .collect();

        panic!(
            "The following protocol ports have no generic service definition:\n{}\n\n\
            Each protocol port needs a generic service (is_generic=true) with either:\n\
            - Pattern::Port(PortType::X)\n\
            - Pattern::Endpoint(PortType::X, ...)\n\
            - Pattern::AnyOf containing one of the above",
            port_list.join("\n")
        );
    }
}
#[test]
fn test_service_patterns_use_appropriate_port_types() {
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    // Build map of port numbers to their PortBase names by iterating
    let well_known_ports: std::collections::HashMap<PortType, String> = PortType::iter()
        .filter_map(|port_base| {
            // Skip Custom variants
            if matches!(port_base, PortType::Custom(_)) {
                None
            } else {
                Some((port_base, format!("PortType::{}", port_base.discriminant())))
            }
        })
        .collect();

    for service in registry {
        let pattern = service.discovery_pattern();
        let service_name = ServiceDefinition::name(&service);

        check_port_usage(&pattern, &well_known_ports, service_name);
    }
}

fn check_port_usage(
    pattern: &Pattern,
    well_known_ports: &std::collections::HashMap<PortType, String>,
    service_name: &str,
) {
    match pattern {
        Pattern::Port(port_base) | Pattern::Endpoint(port_base, .., None) => {
            if let PortType::Custom(_) = port_base
                && let Some(named_constant) = well_known_ports.get(port_base)
            {
                panic!(
                    "Service '{}' uses custom port {} but should use {} instead",
                    service_name, port_base, named_constant
                );
            }
        }
        Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => {
            for p in patterns {
                check_port_usage(p, well_known_ports, service_name);
            }
        }
        Pattern::Not(p) => {
            check_port_usage(p, well_known_ports, service_name);
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_service_patterns_are_specific_enough() {
    let registry = ServiceDefinitionRegistry::all_service_definitions();
    let words_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("words.json");

    // Ensure the words file exists, download if necessary
    if !words_path.exists() {
        eprintln!("Words dictionary not found, downloading...");
        let url =
            "https://raw.githubusercontent.com/dwyl/english-words/master/words_dictionary.json";

        // Create the directory if it doesn't exist
        if let Some(parent) = words_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        // Download and save the file - use async client instead
        let response = reqwest::get(url)
            .await
            .expect("Failed to download words dictionary");
        let content = response.text().await.expect("Failed to read response body");
        std::fs::write(&words_path, content).expect("Failed to write words.json");
        eprintln!("Downloaded words dictionary to {:?}", words_path);
    }

    let words_file = File::open(&words_path).unwrap();
    let reader = BufReader::new(words_file);
    let words_map: HashMap<String, serde_json::Value> = serde_json::from_reader(reader).unwrap();
    let words: HashSet<String> = words_map.into_keys().collect();

    // Get all non-custom PortBase variants by iterating
    let common_ports: Vec<PortType> = PortType::iter()
        // Skip Custom variants
        .filter(|port_base| !matches!(port_base, PortType::Custom(_)))
        .collect();

    for service in registry {
        // Generic services always pass
        if service.is_generic() {
            continue;
        }

        let pattern = service.discovery_pattern();
        let service_name = ServiceDefinition::name(&service);

        check_pattern_specificity(&pattern, &common_ports, service_name, words.clone());
    }
}

fn check_pattern_specificity(
    pattern: &Pattern,
    common_ports: &[PortType],
    service_name: &str,
    words: HashSet<String>,
) {
    match pattern {
        // Port-only patterns on common ports without other criteria = fail
        Pattern::Port(port_base) => {
            if common_ports.contains(port_base) {
                panic!(
                    "Service '{}' uses port-only pattern on common port {} without additional criteria. \
                        This could cause false positives. Consider using:\n\
                        1. Pattern::Endpoint with a unique path/response\n\
                        2. Pattern::AllOf combining port with other criteria\n\
                        3. Mark service as is_generic = true if it's truly generic (ie it represents the implementation of a protocol, not something provided by a specific vendor)",
                    service_name,
                    port_base.discriminant()
                );
            }
        }

        // AnyOf with only port patterns on common ports = fail
        Pattern::AnyOf(patterns) => {
            let all_are_common_port_patterns = patterns.iter().all(|p| {
                if let Pattern::Port(port_base) = p {
                    common_ports.contains(port_base)
                } else {
                    false
                }
            });

            if all_are_common_port_patterns && !patterns.is_empty() {
                panic!(
                    "Service '{}' uses AnyOf with only common port patterns. \
                        This could cause false positives. Use more specific patterns",
                    service_name
                );
            }

            // Check each sub-pattern recursively
            for p in patterns {
                check_pattern_specificity(p, common_ports, service_name, words.clone());
            }
        }

        // Endpoint patterns with common port/path and match strings that could lead to false positive = fail
        Pattern::Endpoint(port_base, path, body_match_string, status_range) => {
            let match_string_lower = body_match_string.to_lowercase();
            let is_short_match_string = match_string_lower.len() < 5;

            // Another service is likely to be listening on this port on other hosts, so need to be more stringent
            let port_is_common = common_ports.contains(port_base);

            // Path is unique/specific enough, even if match string alone is likely to cause false positives
            let path_contains_service_name = path.contains(service_name);

            // Endpoint is probably not unique to service, and other services might respond to it
            let is_common_endpoint = !path_contains_service_name
                && port_is_common
                && (*path == "/" || *path == "/api/" || *path == "/home/");

            // Potential to false positive with dashboards that display service name
            let match_string_is_service_name = match_string_lower == service_name.to_lowercase();

            // Non-compound strings have potential to false positive with dashboards that display service name
            let match_string_is_singular = !match_string_lower.contains(" ")
                && !match_string_lower.contains(".")
                && !match_string_lower.contains("_")
                && !match_string_lower.contains("-")
                && !match_string_lower.contains(",")
                && !match_string_lower.contains("/");

            // Potential to false positive by being found in random strings displayed by other services
            let is_substring_of_any_word = if is_short_match_string {
                words.iter().any(|w| w.contains(&match_string_lower)) && !path_contains_service_name
            } else {
                false
            };

            let expected_range = status_range.as_ref().unwrap_or(&(200..400));
            let range_includes_redirects = expected_range.start < 400 && expected_range.end > 300;

            if is_short_match_string && range_includes_redirects && port_is_common {
                panic!(
                        "Service '{}' uses a match string '{}' that is too short ({} characters) and also accepts redirects.
                        This could cause false positives. Please disallow redirects by passing Some(200..300) as the allowed status range
                        or update the match string to be longer.",
                        service_name,
                        body_match_string,
                        match_string_lower.len(),
                    );
            };

            if is_common_endpoint && match_string_is_service_name {
                panic!(
                        "Service '{}' uses a match string '{}' that is the same as the name of the service. This could cause false positives, \
                        as dashboard services often will contain service names in their own endpoint responses, and as such could get detected as this service
                        Please provide a match string that contains text that distinguishes it from the service name",
                        service_name, body_match_string
                    );
            }

            if is_common_endpoint && match_string_is_singular {
                panic!(
                        "Service '{}' uses a match string '{}' that is a singular word. This could cause false positives, \
                        as dashboard services often will contain service names in their own endpoint responses, and as such could get detected as this service
                        Please provide a compound match string - multiple words separated by one of the following \
                        delimiters: \".\", \"_\", \"/\", \",\", or \"-\"",
                        service_name, body_match_string
                    );
            }

            if is_common_endpoint && is_substring_of_any_word {
                panic!(
                    "Service '{}' uses endpoint pattern at root path '/' on common port {} \
                        with a match string '{}' that is a substring of at least one of a common english word. This could cause false positives. \
                        Consider:\n\
                        1. Use a more specific path (e.g., '/api/status' instead of '/')\n\
                        2. Use a longer, more unique match string\n\
                        3. Use Pattern::AllOf to combine multiple criteria",
                    service_name, port_base, body_match_string
                );
            }
        }

        // Other patterns are generally fine
        _ => {}
    }
}

#[test]
fn test_service_definition_serialization() {
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    // Test that we can serialize and deserialize service definitions
    for service in registry.iter().take(5) {
        // Test first 5 to save time
        // Serialize to JSON
        let json = serde_json::to_string(&service)
            .unwrap_or_else(|_| panic!("Failed to serialize {}", service.name()));

        // Deserialize back
        let deserialized: Box<dyn ServiceDefinition> = serde_json::from_str(&json)
            .unwrap_or_else(|_| panic!("Failed to deserialize {}", service.name()));

        // Verify key fields match
        assert_eq!(
            service.name(),
            deserialized.name(),
            "Name mismatch after serialization"
        );
        assert_eq!(
            service.description(),
            deserialized.description(),
            "Description mismatch after serialization"
        );
    }
}
/// A virtualizer service is created *before* its bindings, so that the subnets and container
/// services naming it as their owner can satisfy their `virtualization_service_id` foreign key
/// (GH #650). That deferral is only safe for non-generic definitions.
///
/// `Service::eq` resolves a generic service by shared port bindings — see the
/// `has_shared_ports` fallback in `services/impl/base.rs`. A generic row persisted with no
/// bindings yet would therefore fail to match its existing counterpart on the next scan and be
/// silently duplicated, which is the same class of bug the owner column was introduced to fix.
/// A non-generic definition resolves on host + definition alone and never reads bindings.
///
/// Asserted over the registry rather than at each call site so the conflict is impossible to
/// introduce, instead of surfacing as duplicate rows in production.
#[test]
fn a_virtualizer_definition_is_never_generic() {
    use crate::server::services::r#impl::definitions::ServiceDefinitionExt;

    let offenders: Vec<&str> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter(|d| {
            ServiceDefinitionExt::virtualization_role(*d).is_some()
                && ServiceDefinitionExt::is_generic(*d)
        })
        .map(|d| ServiceDefinition::name(d))
        .collect();

    assert!(
        offenders.is_empty(),
        "these definitions are both a virtualizer and generic, so they cannot be created \
         ahead of their bindings without duplicating on the next scan: {offenders:?}"
    );
}

#[test]
fn test_service_definition_logo_urls_valid() {
    use crate::server::services::r#impl::definitions::ServiceDefinitionExt;

    let registry = ServiceDefinitionRegistry::all_service_definitions();

    for service in &registry {
        let logo_url = service.logo_url();

        if logo_url.is_empty() {
            assert!(
                !ServiceDefinitionExt::has_logo(service),
                "Service '{}' has has_logo=true but empty logo_url()",
                ServiceDefinition::name(service),
            );
            continue;
        }

        // Local paths must start with /logos/
        if logo_url.starts_with('/') {
            assert!(
                logo_url.starts_with("/logos/"),
                "Service '{}' has local logo URL '{}' that doesn't start with /logos/",
                ServiceDefinition::name(service),
                logo_url
            );
            continue;
        }

        // External URLs must be parseable
        assert!(
            reqwest::Url::parse(logo_url).is_ok(),
            "Service '{}' has invalid logo URL '{}'",
            ServiceDefinition::name(service),
            logo_url
        );
    }
}

#[tokio::test]
async fn test_service_definition_logo_urls_resolve() {
    use backon::{ExponentialBuilder, Retryable};
    use futures::stream::{self, StreamExt};

    let registry = ServiceDefinitionRegistry::all_service_definitions();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let services_to_check: Vec<_> = registry
        .into_iter()
        .filter_map(|service| {
            let logo_url = service.logo_url();
            if logo_url.is_empty() || logo_url.starts_with('/') {
                return None;
            }
            Some((service.name().to_string(), logo_url.to_string()))
        })
        .collect();

    let results: Vec<Result<(), String>> = stream::iter(services_to_check)
        .map(|(service_name, logo_url)| {
            let client = client.clone();
            async move {
                let fetch_with_retry = || {
                    let client = client.clone();
                    let url = logo_url.clone();
                    async move {
                        let response = client
                            .head(&url)
                            .send()
                            .await
                            .map_err(|e| format!("request failed: {}", e))?;

                        if response.status().is_success() {
                            if let Some(content_type) = response.headers().get("content-type") {
                                let content_type_str = content_type.to_str().unwrap_or("");
                                if !content_type_str.starts_with("image/")
                                    && !content_type_str.starts_with("text/plain")
                                {
                                    return Err(format!(
                                        "non-image Content-Type: {}",
                                        content_type_str
                                    ));
                                }
                            }
                            Ok(())
                        } else {
                            Err(format!("returned status {}", response.status()))
                        }
                    }
                };

                fetch_with_retry
                    .retry(
                        ExponentialBuilder::default()
                            .with_max_times(3)
                            .with_min_delay(std::time::Duration::from_millis(500))
                            .with_max_delay(std::time::Duration::from_secs(5)),
                    )
                    .await
                    .map_err(|e| {
                        format!(
                            "Service '{}' logo URL '{}' failed to resolve: {}",
                            service_name, logo_url, e
                        )
                    })
            }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    let failures: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();

    if !failures.is_empty() {
        panic!(
            "Logo URL validation failed for {} service(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[test]
fn test_service_definition_description_starts_with_capital() {
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    for service in registry {
        let description = ServiceDefinition::description(&service);

        // Skip empty descriptions (already caught by another test)
        if description.is_empty() {
            continue;
        }

        let first_char = description.chars().next().unwrap();
        assert!(
            first_char.is_uppercase(),
            "Service '{}' has description '{}' that doesn't start with a capital letter",
            ServiceDefinition::name(&service),
            description
        );
    }
}

#[test]
fn test_all_service_definition_files_have_mod_declaration() {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    // Get the definitions directory path
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let definitions_dir = manifest_dir.join("src/server/services/definitions");

    // Read all .rs files in the definitions directory (excluding mod.rs)
    let rs_files: HashSet<String> = fs::read_dir(&definitions_dir)
        .expect("Failed to read definitions directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            // Only process .rs files that aren't mod.rs
            if path.extension()? == "rs"
                && path.file_stem()? != "mod"
                && path.file_stem()? != "example"
            {
                Some(path.file_stem()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    // Read mod.rs and extract all module declarations
    let mod_rs_path = definitions_dir.join("mod.rs");
    let mod_rs_content = fs::read_to_string(&mod_rs_path).expect("Failed to read mod.rs");

    let declared_modules: HashSet<String> = mod_rs_content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Match lines like "pub mod some_module;"
            if trimmed.starts_with("pub mod ") && trimmed.ends_with(';') {
                let module_name = trimmed
                    .trim_start_matches("pub mod ")
                    .trim_end_matches(';')
                    .trim();
                Some(module_name.to_string())
            } else {
                None
            }
        })
        .collect();

    // Find files without declarations
    let undeclared: Vec<_> = rs_files.difference(&declared_modules).collect();

    // Find declarations without files (shouldn't happen, but check anyway)
    let missing_files: Vec<_> = declared_modules.difference(&rs_files).collect();

    if !undeclared.is_empty() {
        panic!(
            "Service definition files exist but are not declared in mod.rs:\n{}\n\
            Add these lines to mod.rs:\n{}",
            undeclared
                .iter()
                .map(|s| format!("  - {}.rs", s))
                .collect::<Vec<_>>()
                .join("\n"),
            undeclared
                .iter()
                .map(|s| format!("pub mod {};", s))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    if !missing_files.is_empty() {
        panic!(
            "Module declarations in mod.rs have no corresponding file:\n{}",
            missing_files
                .iter()
                .map(|s| format!("  - {}", s))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn test_service_definition_ids_are_stable() {
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::process::Command;

    // Service ID renames that have been intentionally migrated.
    // Format: (old_id, new_id) - these are allowed to differ from origin/main.
    // Once merged to main, these entries become no-ops and can be removed.
    let allowed_migrations: HashSet<(&str, &str)> = [
        ("Dhcp Server", "DHCP Server"),
        ("Dns Server", "DNS Server"),
        ("Hp Printer", "HP Printer"),
    ]
    .into_iter()
    .collect();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let definitions_dir = manifest_dir.join("src/server/services/definitions");

    // Check if we're in a git repository
    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(&manifest_dir)
        .output();

    if git_check.is_err() || !git_check.unwrap().status.success() {
        println!("⚠️  Not in a git repository, skipping ID stability test");
        return;
    }

    // Check if origin/main exists
    let remote_check = Command::new("git")
        .args(["rev-parse", "--verify", "origin/main"])
        .current_dir(&manifest_dir)
        .output();

    if remote_check.is_err() || !remote_check.unwrap().status.success() {
        println!("⚠️  origin/main not found, skipping ID stability test");
        return;
    }

    // Get list of service definition files that existed in origin/main
    let git_files = Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            "origin/main",
            "backend/src/server/services/definitions/",
        ])
        .current_dir(manifest_dir.parent().expect("No parent directory"))
        .output()
        .expect("Failed to list files from origin/main");

    if !git_files.status.success() {
        println!("⚠️  Failed to read origin/main, skipping ID stability test");
        return;
    }

    let committed_files: Vec<String> = String::from_utf8_lossy(&git_files.stdout)
        .lines()
        .filter(|line| {
            line.ends_with(".rs") && !line.ends_with("/mod.rs") && !line.ends_with("/example.rs")
        })
        .map(|line| {
            line.split('/')
                .next_back()
                .unwrap()
                .trim_end_matches(".rs")
                .to_string()
        })
        .collect();

    if committed_files.is_empty() {
        println!("⚠️  No committed service definitions found, skipping ID stability test");
        return;
    }

    // For each committed file, extract the name() value from origin/main
    let mut committed_service_ids: HashMap<String, String> = HashMap::new();

    for filename in &committed_files {
        let file_path = format!("backend/src/server/services/definitions/{}.rs", filename);

        let file_content = Command::new("git")
            .args(["show", &format!("origin/main:{}", file_path)])
            .current_dir(manifest_dir.parent().expect("No parent directory"))
            .output()
            .unwrap_or_else(|_| panic!("Failed to read {} from origin/main", file_path));

        if !file_content.status.success() {
            continue;
        }

        let content = String::from_utf8_lossy(&file_content.stdout);

        // Extract the name() return value
        if let Some(name) = extract_service_name(&content) {
            committed_service_ids.insert(filename.clone(), name);
        }
    }

    // Get current service definitions from registry
    let registry = ServiceDefinitionRegistry::all_service_definitions();

    // Now compare: for each file that existed in origin/main, check if the ID changed
    let mut changed_ids = Vec::new();
    let mut removed_services = Vec::new();

    for (filename, committed_id) in &committed_service_ids {
        // Check if this service still exists in current registry
        let current_has_id = registry.iter().any(|s| s.id() == committed_id);

        if !current_has_id {
            // The ID from origin/main is no longer in the registry
            // Check if the file still exists
            let file_still_exists = definitions_dir.join(format!("{}.rs", filename)).exists();

            if file_still_exists {
                // File exists but ID changed - extract current ID
                let current_file_content =
                    std::fs::read_to_string(definitions_dir.join(format!("{}.rs", filename))).ok();

                if let Some(content) = current_file_content
                    && let Some(new_id) = extract_service_name(&content)
                    && new_id != *committed_id
                {
                    // Check if this is an allowed migration
                    let is_allowed =
                        allowed_migrations.contains(&(committed_id.as_str(), new_id.as_str()));
                    if !is_allowed {
                        changed_ids.push(format!(
                            "  - File '{}.rs': ID changed from '{}' to '{}'",
                            filename, committed_id, new_id
                        ));
                    }
                }
            } else {
                removed_services.push(format!("  - '{}.rs' (ID was '{}')", filename, committed_id));
            }
        }
    }

    if !changed_ids.is_empty() {
        panic!(
            "Service definition IDs have changed (this breaks database compatibility):\n{}\n\n\
            Service IDs (derived from name() method) must remain stable once committed to origin/main.\n\
            Changing a service name breaks existing databases that reference the old ID.\n\n\
            If you must rename a service, you need to:\n\
            1. Provide a database migration script\n\
            2. Update all service records in the database to use the new ID\n\
            3. Document this as a breaking change",
            changed_ids.join("\n")
        );
    }

    if !removed_services.is_empty() {
        println!(
            "⚠️  Service definition files were removed:\n{}\n\
            If this is intentional, ensure a migration handles orphaned records.",
            removed_services.join("\n")
        );
    }
}

// Helper function to extract service name from Rust source code
fn extract_service_name(content: &str) -> Option<String> {
    // Use a simple regex-like approach to find: fn name(&self) ... { ... "name" ... }
    // We need to handle whitespace and newlines between tokens

    // Find "fn name(&self)"
    let fn_pos = content.find("fn name(&self)")?;

    // From there, find the opening brace
    let after_fn = &content[fn_pos..];
    let brace_pos = after_fn.find('{')?;

    // Now find the first string literal after the brace
    let after_brace = &after_fn[brace_pos + 1..];

    // Find first quote
    let first_quote = after_brace.find('"')?;
    let after_first_quote = &after_brace[first_quote + 1..];

    // Find closing quote (need to handle escaped quotes)
    let mut end_pos = 0;
    let chars: Vec<char> = after_first_quote.chars().collect();

    for i in 0..chars.len() {
        if chars[i] == '"' {
            // Check if it's escaped
            if i > 0 && chars[i - 1] == '\\' {
                continue;
            }
            end_pos = i;
            break;
        }
    }

    if end_pos == 0 {
        return None;
    }

    let name: String = chars[..end_pos].iter().collect();
    Some(name)
}

/// Every [`ClientProbe`] variant must have something that produces it.
///
/// A variant nothing emits gives its service definition a pattern that can never match, and the
/// definition looks entirely healthy while doing nothing — which is the state
/// `services/definitions/gnmi.rs` has been in. Both producers are `inventory` collections
/// populated at link time, so no `const` can see across them; this is the assertion that can.
#[test]
fn every_client_probe_variant_has_a_producer() {
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    use crate::daemon::utils::app_probe::all_app_probes;
    use crate::server::services::r#impl::patterns::ClientProbe;

    // Credentialed integrations build their `ProbeSuccess` inside `probe()` at runtime rather
    // than declaring it, so there is nothing to enumerate and they are named here.
    let from_integrations = [
        ClientProbe::Snmp,
        ClientProbe::Docker,
        ClientProbe::Podman,
        ClientProbe::UnifiController,
        ClientProbe::InstantOn,
    ];

    // Known to have no producer. gNMI's definition matches on `ClientResponse(Gnmi)` and nothing
    // emits it, so that service can never be detected. Listed rather than the test being weakened,
    // so a *new* orphan still fails here.
    let known_unproduced = [ClientProbe::Gnmi];

    let from_app_probes: HashSet<ClientProbe> = all_app_probes()
        .iter()
        .filter_map(|probe| probe.client_probe())
        .collect();

    let orphans: Vec<ClientProbe> = ClientProbe::iter()
        .filter(|c| !from_app_probes.contains(c))
        .filter(|c| !from_integrations.contains(c))
        .filter(|c| !known_unproduced.contains(c))
        .collect();

    assert!(
        orphans.is_empty(),
        "these ClientProbe variants have no producer, so any service definition matching on them \
         can never match: {orphans:?}\n\
         Either emit them from an AppProbe or a DiscoveryIntegration's probe(), or remove them."
    );
}

/// A definition's probe and its discovery pattern have to agree about the port.
///
/// `probe_pattern` derives the pattern from the probe, so the two cannot drift where it is used —
/// this catches a definition that hand-writes a pattern for one port while handing out a probe for
/// another, which would scan a port nothing probes and probe a port nothing scans.
#[test]
fn a_definitions_probe_port_is_a_port_its_pattern_scans() {
    use crate::server::services::definitions::ServiceDefinitionRegistry;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    for definition in ServiceDefinitionRegistry::all_service_definitions() {
        let scanned = definition.discovery_pattern().ports();
        for probe in definition.app_probes() {
            assert!(
                scanned.contains(&probe.port()),
                "{} probes {:?} but its discovery pattern scans {:?}",
                ServiceDefinition::name(&definition),
                probe.port(),
                scanned
            );
        }
    }
}

/// No definition names a service on the strength of a completed TCP connection alone, unless it
/// says why it cannot do better.
///
/// A middlebox in the path — a firewall session helper, a transparent proxy, an IPS — completes the
/// handshake on behalf of addresses that hold no device. A definition resting on the connect alone
/// then invents a service there, which is how a FortiGate SIP session helper became a "SIP Server"
/// on every remote VLAN a customer scanned. A definition that reads a protocol response, an HTTP
/// body or a header cannot be fooled that way, because the middlebox would have to speak the
/// protocol to satisfy it.
///
/// The two sides are derived independently and compared: the left from each pattern's own shape via
/// [`Pattern::matches_on_connect_alone`], the right from what each definition declares. That is what
/// makes this maintenance-free — writing a probe or an endpoint match drops a definition off the
/// left side automatically, and a new bare-port definition lands on it without anyone remembering
/// to update a list.
#[test]
fn connect_only_definitions_are_declared() {
    use crate::server::services::definitions::ServiceDefinitionRegistry;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    let derived: HashSet<&'static str> = ServiceDefinitionRegistry::connect_only_definitions()
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let declared: HashSet<&'static str> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter(|d| d.connect_only_rationale().is_some())
        .map(|d| ServiceDefinition::name(d))
        .collect();

    let undeclared: BTreeSet<&str> = derived.difference(&declared).copied().collect();
    let stale: BTreeSet<&str> = declared.difference(&derived).copied().collect();

    assert!(
        undeclared.is_empty(),
        "these definitions match on a bare TCP connect, which any middlebox in the path can \
         satisfy on behalf of an address that holds no device: {undeclared:?}\n\
         Give each one evidence it can check — an `app_probes()` entry, or a `Pattern::Endpoint` / \
         `Pattern::Header` that matches a body or header — or declare \
         `connect_only_rationale()` saying which protocol fact prevents that."
    );

    assert!(
        stale.is_empty(),
        "these definitions declare a `connect_only_rationale()` they no longer need, because their \
         pattern already validates what answered: {stale:?}\n\
         Remove the declaration; the exception set is meant to shrink."
    );
}

/// Every definition can actually be matched by something the scan collects.
///
/// A definition whose evidence no scan stage gathers is dead weight that looks like coverage.
/// `openvpn`, `strongswan` and `wireguard` sat in the registry for years matching on a UDP port
/// with no probe behind it, and nothing said so: UDP ports are only ever reported open by a
/// registered [`AppProbe`], so those patterns could never be satisfied. The connect-only guard
/// above would not have caught them, because their defect is unmatchability rather than weak
/// evidence.
///
/// TCP ports are reachable by the connect scan; UDP ports only through a probe. A definition
/// matching on something other than ports (an endpoint, mDNS, a controller inventory, a manual
/// assignment via `Pattern::None`) has no port to check and passes.
#[test]
fn every_definition_can_be_matched() {
    use crate::daemon::utils::app_probe::all_app_probes;
    use crate::server::services::definitions::ServiceDefinitionRegistry;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    let probed_ports: HashSet<_> = all_app_probes().iter().map(|p| p.port()).collect();

    let unreachable: BTreeSet<String> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .flat_map(|definition| {
            let name = ServiceDefinition::name(definition);
            definition
                .discovery_pattern()
                .ports()
                .into_iter()
                .filter(|port| port.is_udp() && !probed_ports.contains(port))
                .map(move |port| format!("{name} ({port})"))
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(
        unreachable.is_empty(),
        "these definitions depend on a UDP port no AppProbe covers, so nothing ever reports it \
         open and the definition can never match: {unreachable:?}\n\
         Write a probe for the port, or change the pattern to something the scan does collect."
    );
}
