//! Signup-time disposable-domain check for an email address.
//!
//! Refuses a domain listed by the `mailchecker` crate or by the bundled
//! `disposable-email-domains` list (`assets/disposable-email-domains.txt`, refreshed
//! at each release by `scripts/refresh-vendored-data.sh`). A listed domain
//! also covers its subdomains. A domain in `assets/disposable-email-allowlist.txt`
//! is accepted whatever either list says.

use std::collections::HashSet;
use std::sync::LazyLock;

use email_address::EmailAddress;

static DISPOSABLE_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    parse_domain_list(include_str!("../../../assets/disposable-email-domains.txt"))
});

static ALLOWED_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    parse_domain_list(include_str!(
        "../../../assets/disposable-email-allowlist.txt"
    ))
});

/// One domain per line; blank lines and `#` comments are skipped.
fn parse_domain_list(text: &'static str) -> HashSet<&'static str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Whether sign-up should refuse this address as disposable.
pub fn is_disposable_email(email: &EmailAddress) -> bool {
    is_disposable(email, &DISPOSABLE_DOMAINS, &ALLOWED_DOMAINS)
}

fn is_disposable(
    email: &EmailAddress,
    disposable: &HashSet<&str>,
    allowed: &HashSet<&str>,
) -> bool {
    let domain = email.domain().to_ascii_lowercase();
    let candidates: Vec<&str> = domain_and_parents(&domain).collect();
    if candidates.iter().any(|d| allowed.contains(d)) {
        return false;
    }
    !mailchecker::is_valid(email.as_str()) || candidates.iter().any(|d| disposable.contains(d))
}

/// The domain and each parent, stopping before the bare TLD:
/// `a.b.example.com` yields `a.b.example.com`, `b.example.com`, `example.com`.
fn domain_and_parents(domain: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(domain), |d| d.split_once('.').map(|(_, rest)| rest))
        .filter(|d| d.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn email(address: &str) -> EmailAddress {
        EmailAddress::from_str(address).unwrap()
    }

    #[test]
    fn listed_domain_and_its_subdomains_are_refused() {
        for address in [
            "user@goldvester.com",
            "user@x.goldvester.com",
            "user@GoldVester.com",
        ] {
            assert!(is_disposable_email(&email(address)), "{address}");
        }
    }

    #[test]
    fn mailchecker_only_domains_are_refused() {
        // Listed by mailchecker but absent from the bundled list, so this exercises the
        // mailchecker half of the check on its own.
        let address = "user@spamgourmet.com";
        assert!(!DISPOSABLE_DOMAINS.contains("spamgourmet.com"));
        assert!(!mailchecker::is_valid(address));
        assert!(is_disposable_email(&email(address)));
    }

    #[test]
    fn mailchecker_disposable_domains_are_refused() {
        for address in [
            "user@mailinator.com",
            "user@guerrillamail.com",
            "user@yopmail.com",
            "user@sharklasers.com",
            "user@trashmail.com",
        ] {
            assert!(is_disposable_email(&email(address)), "{address}");
        }
    }

    #[test]
    fn ordinary_domains_are_accepted() {
        for address in [
            "user@gmail.com",
            "user@scanopy.net",
            "user@microsoft.com",
            // Hosted on NameSilo forwarding like goldvester.com, but not disposable.
            "user@goldvest.com",
            "user@yahoo.co.jp",
            "user@yahoo.co.uk",
            "user@yahoo.ca",
            "user@hotmail.co.uk",
            "user@hotmail.it",
            "user@outlook.com",
            "user@protonmail.com",
        ] {
            assert!(!is_disposable_email(&email(address)), "{address}");
        }
    }

    #[test]
    fn allowlist_overrides_both_lists() {
        let allowed = HashSet::from(["goldvester.com", "mailinator.com"]);
        for address in [
            "user@goldvester.com",
            "user@x.goldvester.com",
            "user@mailinator.com",
        ] {
            assert!(
                !is_disposable(&email(address), &DISPOSABLE_DOMAINS, &allowed),
                "{address}"
            );
        }
    }

    #[test]
    fn parent_walk_stops_before_the_tld() {
        let disposable = HashSet::from(["com"]);
        assert!(!is_disposable(
            &email("user@example.com"),
            &disposable,
            &HashSet::new()
        ));
    }
}
