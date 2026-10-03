//! What the installer prints when an install command's api key matches none of the daemons
//! already on the host. Pure text, kept apart from the terminal handling so it can be tested.

use super::Installed;

/// One install as a numbered list line: its server-assigned name once connected, else its slot.
/// The service id is added only where it says something the rest of the line doesn't.
pub(super) fn installed_line(entry: &Installed) -> String {
    let service_id = entry.service_id();
    match (&entry.name, entry.daemon_id) {
        (Some(name), Some(_)) if *name == service_id => name.clone(),
        (Some(name), Some(_)) => format!("{name} (service {service_id})"),
        _ => format!(
            "{} (never connected to a server; service {service_id})",
            entry.slot
        ),
    }
}

/// The numbered list of what is already installed on this host.
pub(super) fn installed_summary(installed: &[Installed]) -> String {
    let count = installed.len();
    let noun = if count == 1 { "daemon" } else { "daemons" };
    let mut text = format!("This host already has {count} Scanopy {noun} installed:\n\n");
    for (index, entry) in installed.iter().enumerate() {
        text.push_str(&format!("  {}) {}\n", index + 1, installed_line(entry)));
    }
    text
}

/// The question [`choose_ambiguous_target`] asks, up to and including the `Choice [n]:` prompt.
/// Every choice is its own line, so the answers are exactly `n` and the listed numbers.
pub(super) fn ambiguous_target_prompt(installed: &[Installed]) -> String {
    let mut text = installed_summary(installed);
    text.push_str(
        "\nThe API key in this command is for a different daemon. What do you want to do?\n\n",
    );
    text.push_str("  n) Install it as an additional daemon on this host\n");
    if let [_] = installed {
        text.push_str(
            "  1) Replace daemon 1 with it. Use this if daemon 1's install failed or you created\n     \
             a new daemon in Scanopy to replace it. Delete the old daemon in Scanopy afterwards.\n",
        );
    } else {
        for number in 1..=installed.len() {
            text.push_str(&format!("  {number}) Replace daemon {number} with it\n"));
        }
        text.push_str(
            "\n  Replace a daemon if its install failed or you created a new daemon in Scanopy to\n  \
             replace it. Delete the old daemon in Scanopy afterwards.\n",
        );
    }
    text.push_str("\nChoice [n]: ");
    text
}
