use super::{Body, Content, Email, EmailCategory, EmailPreference, links};

/// Delivers a ready-to-run daemon install command for the recipient's OS.
pub struct InstallCommand<'a> {
    pub install_command: &'a str,
    pub os: &'a str,
}

impl Email for InstallCommand<'_> {
    fn subject(&self) -> String {
        "Your Daemon Install Command".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "install_command"
    }

    fn body_html(&self) -> String {
        let code_block = format!(
            r#"                            <div style="margin: 0 0 20px 0; padding: 16px; background-color: #1e293b; border-radius: 6px; overflow-x: auto;">
                                <pre style="margin: 0; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 13px; line-height: 20px; color: #e2e8f0; white-space: pre-wrap; word-break: break-all;">{}</pre>
                            </div>
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Copy and paste this command into your terminal. The daemon will download, install, and start automatically.</p>
"#,
            self.install_command
        );
        Body::new()
            .content(
                Content::new()
                    .heading("Install Command")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Here's the daemon install command you requested. Run it on your {} machine to set up your Scanopy daemon.",
                        self.os
                    ))
                    .raw(&code_block),
            )
            .cta(links::APP_HOME, "Open Scanopy")
            .render()
    }
}
