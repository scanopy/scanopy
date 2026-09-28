//! `SCANOPY_<NAME>_FILE` support: read a config value from a file whose path is in the env.
//!
//! Docker secrets, Kubernetes and Portainer mount secrets as files and pass their paths in
//! `<VAR>_FILE`. Before a binary loads its config, [`apply_file_env_vars`] reads each such file and
//! sets `SCANOPY_<NAME>` to its contents, so Figment and every direct `std::env::var` reader see the
//! value with no per-field handling.

use anyhow::{Result, anyhow};
use serde::Serialize;
use std::collections::HashSet;

const PREFIX: &str = "SCANOPY_";
const SUFFIX: &str = "_FILE";

/// Resolve every `SCANOPY_<NAME>_FILE` in the process env into `SCANOPY_<NAME>`.
///
/// `T` is the binary's config struct. A variable whose full name is itself a config field (the
/// daemon's `SCANOPY_LOG_FILE` is `log_file`) is a plain value, not a file reference; that field
/// takes a file through `SCANOPY_LOG_FILE_FILE`.
pub fn apply_file_env_vars<T: Serialize + Default>() -> Result<()> {
    let fields = config_field_names(&T::default())?;
    for (key, value) in resolve_file_vars(std::env::vars(), &fields)? {
        tracing::debug!("{key} read from {key}{SUFFIX}");
        // SAFETY: runs during startup while config loads, before any task that reads the env is
        // spawned. `dotenvy::dotenv()` mutates the env at the same point.
        unsafe { std::env::set_var(&key, value) };
    }
    Ok(())
}

fn config_field_names<T: Serialize>(config: &T) -> Result<HashSet<String>> {
    match serde_json::to_value(config)? {
        serde_json::Value::Object(map) => Ok(map.into_iter().map(|(k, _)| k).collect()),
        _ => Err(anyhow!("config does not serialize to an object")),
    }
}

/// Returns the `(SCANOPY_<NAME>, contents)` pairs to set. Errors name the variable and path, never
/// the file contents.
fn resolve_file_vars(
    vars: impl IntoIterator<Item = (String, String)>,
    fields: &HashSet<String>,
) -> Result<Vec<(String, String)>> {
    let vars: Vec<(String, String)> = vars.into_iter().collect();
    let is_set = |name: &str| vars.iter().any(|(k, _)| k == name);

    let mut resolved = Vec::new();
    for (key, path) in &vars {
        let Some(name) = key
            .strip_prefix(PREFIX)
            .and_then(|n| n.strip_suffix(SUFFIX))
            .filter(|n| !n.is_empty())
        else {
            continue;
        };
        if fields.contains(&format!("{name}{SUFFIX}").to_lowercase()) {
            continue;
        }

        let target = format!("{PREFIX}{name}");
        if is_set(&target) {
            return Err(anyhow!(
                "{target} and {key} are both set; set only one of them"
            ));
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("{key}: cannot read file {path}: {e}"))?;
        resolved.push((target, contents.trim_end_matches(['\r', '\n']).to_string()));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn secret_file(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file
    }

    fn var(key: &str, value: &str) -> (String, String) {
        (key.to_string(), value.to_string())
    }

    fn path(file: &NamedTempFile) -> String {
        file.path().to_str().unwrap().to_string()
    }

    #[test]
    fn value_read_from_file() {
        let file = secret_file("hunter2");
        let resolved = resolve_file_vars(
            [var("SCANOPY_SMTP_PASSWORD_FILE", &path(&file))],
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(resolved, vec![var("SCANOPY_SMTP_PASSWORD", "hunter2")]);
    }

    #[test]
    fn trailing_newlines_trimmed_trailing_spaces_kept() {
        let lf = secret_file("secret\n");
        let crlf = secret_file("pass word \r\n");
        let resolved = resolve_file_vars(
            [
                var("SCANOPY_A_FILE", &path(&lf)),
                var("SCANOPY_B_FILE", &path(&crlf)),
            ],
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(
            resolved,
            vec![var("SCANOPY_A", "secret"), var("SCANOPY_B", "pass word ")]
        );
    }

    #[test]
    fn both_plain_and_file_set_is_an_error_naming_the_variable() {
        let file = secret_file("from-file");
        let err = resolve_file_vars(
            [
                var("SCANOPY_DATABASE_URL", "postgres://plain"),
                var("SCANOPY_DATABASE_URL_FILE", &path(&file)),
            ],
            &HashSet::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("SCANOPY_DATABASE_URL_FILE"), "{err}");
        assert!(err.contains("SCANOPY_DATABASE_URL "), "{err}");
    }

    #[test]
    fn missing_file_is_an_error_naming_variable_and_path() {
        let err = resolve_file_vars(
            [var(
                "SCANOPY_METRICS_TOKEN_FILE",
                "/nonexistent/scanopy/token",
            )],
            &HashSet::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("SCANOPY_METRICS_TOKEN_FILE"), "{err}");
        assert!(err.contains("/nonexistent/scanopy/token"), "{err}");
    }

    #[test]
    fn non_utf8_file_error_omits_contents() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"topsecret\xff").unwrap();
        let err = resolve_file_vars(
            [var("SCANOPY_STRIPE_SECRET_FILE", &path(&file))],
            &HashSet::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("SCANOPY_STRIPE_SECRET_FILE"), "{err}");
        assert!(!err.contains("topsecret"), "{err}");
    }

    #[test]
    fn field_whose_name_ends_in_file_is_a_plain_value() {
        #[derive(Serialize, Default)]
        struct Config {
            log_file: Option<String>,
        }
        let fields = config_field_names(&Config::default()).unwrap();
        let log = secret_file("/var/log/scanopy.log\n");

        let resolved = resolve_file_vars(
            [
                var("SCANOPY_LOG_FILE", "/var/log/daemon.log"),
                var("SCANOPY_LOG_FILE_FILE", &path(&log)),
            ],
            &fields,
        );
        // SCANOPY_LOG_FILE is the field itself; SCANOPY_LOG_FILE_FILE conflicts with it.
        assert!(
            resolved
                .unwrap_err()
                .to_string()
                .contains("SCANOPY_LOG_FILE_FILE")
        );

        let resolved =
            resolve_file_vars([var("SCANOPY_LOG_FILE", "/var/log/daemon.log")], &fields).unwrap();
        assert!(resolved.is_empty());

        let resolved =
            resolve_file_vars([var("SCANOPY_LOG_FILE_FILE", &path(&log))], &fields).unwrap();
        assert_eq!(
            resolved,
            vec![var("SCANOPY_LOG_FILE", "/var/log/scanopy.log")]
        );
    }

    #[test]
    fn other_prefixes_ignored() {
        let resolved = resolve_file_vars(
            [
                var("POSTGRES_PASSWORD_FILE", "/nonexistent"),
                var("SCANOPY__FILE", "/nonexistent"),
            ],
            &HashSet::new(),
        )
        .unwrap();
        assert!(resolved.is_empty());
    }
}
