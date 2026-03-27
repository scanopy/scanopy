// Email template constants

pub const EMAIL_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Scanopy</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #f5f5f5;">
    <table role="presentation" style="width: 100%; border-collapse: collapse; background-color: #f5f5f5;">
        <tr>
            <td align="center" style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; width: 100%; border-collapse: collapse; background-color: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);">
                    <!-- Header with Logo -->
                    <tr>
                        <td align="center" style="padding: 40px 40px 30px 40px;">
                            <img src="https://cdn.jsdelivr.net/gh/scanopy/scanopy@main/media/logo.png" alt="Scanopy" style="width: 80px; height: 80px; display: block;">
                        </td>
                    </tr>
"#;

pub const EMAIL_FOOTER: &str = r#"                    <!-- Footer -->
                    <tr>
                        <td align="center" style="padding: 30px 40px 20px 40px; background-color: #f9fafb; border-radius: 0 0 8px 8px;">
                            <!-- Social Links -->
                            <table role="presentation" style="margin: 0 auto 20px auto; border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 0 10px;">
                                        <a href="https://discord.com/invite/b7ffQr8AcZ" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/discord.png" alt="Discord" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                    <td style="padding: 0 10px;">
                                        <a href="https://github.com/scanopy/scanopy" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/github.png" alt="GitHub" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                </tr>
                            </table>
                            
                            <p style="margin: 0; font-size: 12px; line-height: 18px; color: #9ca3af;">© {current_year} Scanopy. All rights reserved.</p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
"#;

pub const PASSWORD_RESET_TITLE: &str = "Scanopy Password Reset";

pub const PASSWORD_RESET_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Reset Your Password</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">We received a request to reset your password for your Scanopy account. Click the button below to create a new password:</p>
                        </td>
                    </tr>
                    
                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{reset_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Reset Password</a>
                        </td>
                    </tr>
                    
                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{reset_url}</p>
                        </td>
                    </tr>
                    
                    <!-- Security Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This password reset link will expire in 24 hours. If you didn't request a password reset, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;

pub const INVITE_LINK_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">You've Been Invited to Scanopy</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{inviter_name} has invited you to join their Scanopy instance to visualize and explore their network infrastructure.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Click the button below to accept the invitation and create your account:</p>
                        </td>
                    </tr>
                    
                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{invite_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Accept Invitation</a>
                        </td>
                    </tr>
                    
                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{invite_url}</p>
                        </td>
                    </tr>
                    
                    <!-- Expiration Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This invitation link will expire in 7 days. If you didn't expect this invitation, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;

// ============================================================================
// Billing Templates
// ============================================================================

pub const TRIAL_STARTED_TITLE: &str = "Welcome to Scanopy - Your Trial Has Started";

pub const TRIAL_STARTED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Welcome to Scanopy {plan_name} {billing_period}!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your trial of the {plan_name} {billing_period} plan has started. You have full access to all features for the next {trial_days} days.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">After your trial, you'll be billed {base_price}*. No credit card is required during the trial — add a payment method anytime from your Settings page to continue after the trial ends.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Add Payment Method</a>
                        </td>
                    </tr>
"#;

pub const TRIAL_ENDING_TITLE: &str = "Your Scanopy Trial Ends in 3 Days";

pub const TRIAL_ENDING_BODY_NO_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial ends in 3 days. To keep all your features and data, add a payment method ({base_price}*) before the trial expires.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If no payment method is added, your account will be downgraded to the Free plan, which includes up to 25 hosts with manual discovery only.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Add Payment Method</a>
                        </td>
                    </tr>
"#;

pub const TRIAL_ENDING_BODY_HAS_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial ends in 3 days. You'll be billed {base_price}* for your {plan_name} {billing_period} plan at the end of the trial period.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Billing</a>
                        </td>
                    </tr>
"#;

pub const TRIAL_EXPIRED_TITLE: &str = "Your Scanopy Trial Has Ended";

pub const TRIAL_EXPIRED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Has Ended</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial has ended and your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can still use Scanopy with up to 25 hosts and manual discovery. Upgrade anytime to restore scheduled discovery and higher limits.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Upgrade Plan</a>
                        </td>
                    </tr>
"#;

pub const PLAN_CHANGED_TITLE: &str = "Your Scanopy Plan Has Changed";

pub const PLAN_CHANGED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Plan Updated</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy plan has been changed to {plan_name}. The change takes effect immediately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;

pub const SUBSCRIPTION_CANCELLED_TITLE: &str = "Your Scanopy Subscription Has Been Cancelled";

pub const SUBSCRIPTION_CANCELLED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Subscription Cancelled</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy subscription has been cancelled. Your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can continue using Scanopy with up to 25 hosts and manual discovery. Resubscribe anytime from your Settings page.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Resubscribe</a>
                        </td>
                    </tr>
"#;

pub const PAYMENT_METHOD_ADDED_TITLE: &str = "Payment Method Added - Scanopy";

pub const PAYMENT_METHOD_ADDED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Method Added</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">A payment method has been added to your Scanopy account. Your subscription will continue automatically when the trial ends.</p>
                        </td>
                    </tr>
"#;

pub const TRIAL_CONVERTED_TITLE: &str = "Your Scanopy Subscription is Now Active";

pub const TRIAL_CONVERTED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Subscription is Active!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial has ended and your subscription is now active. You'll be billed {base_price}* going forward.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;

pub const USAGE_SUMMARY_TITLE: &str = "Your Scanopy Invoice — {period}";

pub const USAGE_SUMMARY_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Monthly Billing Summary</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's a summary of your Scanopy billing for {period}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Invoice date: {invoice_date}</p>

                            <!-- Line Items Table -->
                            <table role="presentation" style="width: 100%; border-collapse: collapse; margin: 0 0 20px 0;">
                                <tr>
                                    <td style="padding: 8px 0; border-bottom: 2px solid #1a1a1a; font-size: 14px; font-weight: 600; color: #1a1a1a;">Description</td>
                                    <td style="padding: 8px 0; border-bottom: 2px solid #1a1a1a; font-size: 14px; font-weight: 600; color: #1a1a1a; text-align: right;">Amount</td>
                                </tr>
                                {line_items_html}
                                <tr>
                                    <td style="padding: 12px 0 0 0; font-size: 16px; font-weight: 600; color: #1a1a1a;">Total</td>
                                    <td style="padding: 12px 0 0 0; font-size: 16px; font-weight: 600; color: #1a1a1a; text-align: right;">{total}</td>
                                </tr>
                            </table>
                            <p style="margin: 0; font-size: 14px; line-height: 20px; color: #6b7280;">Questions? Please reach out to <a href="mailto:billing@scanopy.net" style="color: #2563eb; text-decoration: none;">billing@scanopy.net</a></p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Billing</a>
                        </td>
                    </tr>
"#;

// ============================================================================
// Onboarding Templates
// ============================================================================

pub const DISCOVERY_GUIDE_FREE_TITLE: &str =
    "Your Daemon is Connected - Start Your First Discovery";

pub const DISCOVERY_GUIDE_FREE_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Daemon is Connected!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Great news — your daemon <strong>{daemon_name}</strong> just registered on <strong>{network_name}</strong>. Scanopy is now running an initial discovery to map out your network.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's what happens next:</p>
                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and interfaces are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
                                <li><strong>Docker discovery:</strong> If your daemon has access to the Docker socket, it'll also discover all your containers — images, ports, networks, and labels — automatically.</li>
                            </ul>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">The first discovery runs automatically, but you'll need to trigger subsequent sessions manually. To keep your network map up to date, consider upgrading to a plan with scheduled discovery.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Explore Plans</a>
                        </td>
                    </tr>
"#;

pub const DISCOVERY_GUIDE_PAID_TITLE: &str = "Your Daemon is Connected - Discovery is Running";

pub const DISCOVERY_GUIDE_PAID_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Daemon is Connected!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Great news — your daemon <strong>{daemon_name}</strong> just registered on <strong>{network_name}</strong>. Scanopy is now running an initial discovery to map out your network.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's what happens next:</p>
                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and interfaces are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
                                <li><strong>Scheduled discovery:</strong> Your plan includes daily scheduled discovery — your network documentation stays up to date automatically.</li>
                                <li><strong>Docker discovery:</strong> If your daemon has access to the Docker socket, it'll also discover all your containers — images, ports, networks, and labels — automatically.</li>
                            </ul>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;

pub const TOPOLOGY_READY_TITLE: &str = "Your Network Topology is Ready";

pub const TOPOLOGY_READY_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Topology is Ready!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your first discovery on <strong>{network_name}</strong> has completed. Scanopy found <strong>{host_count} hosts</strong> and <strong>{service_count} services</strong>.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your interactive topology map is now available — open Scanopy to explore your network visually.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;"><strong>Quick tips:</strong> Drag nodes to rearrange your layout, click any host to inspect its services and details, and use the export button to save your map.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/#topology" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Topology</a>
                        </td>
                    </tr>
"#;

pub const PLAN_LIMIT_APPROACHING_TITLE: &str = "You're Approaching Your {limit_type} Limit";

pub const PLAN_LIMIT_APPROACHING_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Approaching Plan Limit</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You're using <strong>{current_count}</strong> of your <strong>{limit}</strong> included {limit_type} on the {plan_name} plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{limit_message}</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal={cta_modal}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">{cta_label}</a>
                        </td>
                    </tr>
"#;

pub const PLAN_LIMIT_REACHED_TITLE: &str = "You've Reached Your {limit_type} Limit";

pub const PLAN_LIMIT_REACHED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Plan Limit Reached</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You've reached <strong>{current_count}</strong> of your <strong>{limit}</strong> included {limit_type} on the {plan_name} plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{limit_message}</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal={cta_modal}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">{cta_label}</a>
                        </td>
                    </tr>
"#;

pub const PAYMENT_FAILED_TITLE: &str = "Payment Failed - Action Required";

pub const PAYMENT_FAILED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Failed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your recent payment for Scanopy failed. Please update your payment method to avoid service interruption.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you believe this is an error, check with your bank or try a different payment method.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Update Payment Method</a>
                        </td>
                    </tr>
"#;

pub const PAYMENT_ACTION_REQUIRED_TITLE: &str = "Payment Requires Authentication";

pub const PAYMENT_ACTION_REQUIRED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Requires Authentication</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your recent payment for Scanopy requires additional authentication (3D Secure). Please complete the verification to continue your subscription.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Complete Verification</a>
                        </td>
                    </tr>
"#;

// ============================================================================
// Daemon Standby Templates
// ============================================================================

pub const DAEMON_STANDBY_TITLE: &str = "Your Daemon Has Been Put on Standby";

pub const DAEMON_STANDBY_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Daemon on Standby</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon <strong>{daemon_name}</strong> on <strong>{network_name}</strong> has been placed on <a href="https://scanopy.net/docs/reference/daemon-status/" style="color: #2563eb; text-decoration: none;">standby</a> because it hasn't completed a discovery session in over 30 days. While on standby, scheduled discoveries targeting this daemon will be skipped.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">To resume:</p>
                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Ensure your daemon is running and connected (<a href="https://scanopy.net/docs/setting-up-daemons/troubleshooting/" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                                <li>Manually start a discovery by pressing the Play button on the Discoveries page.</li>
                            </ol>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon will come off standby and scheduled discoveries will resume automatically.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/#discovery-scheduled" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Queue Discovery</a>
                        </td>
                    </tr>
"#;

// ============================================================================
// Daemon Unreachable Templates
// ============================================================================

pub const DAEMON_UNREACHABLE_TITLE: &str = "Your Daemon Is Unreachable";

pub const DAEMON_UNREACHABLE_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Daemon Unreachable</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon <strong>{daemon_name}</strong> on <strong>{network_name}</strong> is unreachable. Scheduled discoveries targeting this daemon will be skipped until connectivity is restored.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">To resolve:</p>
                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Check that the daemon host is online and the daemon process is running.</li>
                                <li>Verify network connectivity between the server and daemon (<a href="https://scanopy.net/docs/setting-up-daemons/troubleshooting/" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                            </ol>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Once connectivity is restored, your daemon will automatically resume and scheduled discoveries will run again.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/#daemons" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Daemons</a>
                        </td>
                    </tr>
"#;

// ============================================================================
// Account Change Notification Templates
// ============================================================================

pub const PASSWORD_CHANGED_TITLE: &str = "Your Scanopy Password Was Changed";

pub const PASSWORD_CHANGED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Password Changed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy password was changed on {timestamp}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you made this change, no action is needed. If you didn't change your password, please reset it immediately and contact support.</p>
                        </td>
                    </tr>
"#;

pub const OIDC_LINKED_TITLE: &str = "{provider_name} Login Connected - Scanopy";

pub const OIDC_LINKED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Login Method Connected</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {provider_name} account has been linked to your Scanopy account. You can now sign in using {provider_name}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you didn't make this change, please sign in to your account and unlink this provider from Settings.</p>
                        </td>
                    </tr>
"#;

pub const OIDC_UNLINKED_TITLE: &str = "{provider_name} Login Disconnected - Scanopy";

pub const OIDC_UNLINKED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Login Method Disconnected</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {provider_name} account has been unlinked from your Scanopy account. You can no longer sign in using {provider_name}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you didn't make this change, please sign in to your account and review your security settings.</p>
                        </td>
                    </tr>
"#;

pub const EMAIL_CHANGED_OLD_TITLE: &str = "Your Scanopy Email Was Changed";

pub const EMAIL_CHANGED_OLD_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Email Address Changed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">The email address on your Scanopy account was changed to <strong>{new_email}</strong>.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you made this change, no action is needed. If you didn't request this change, please contact support immediately.</p>
                        </td>
                    </tr>
"#;

// ============================================================================
// Auth Templates
// ============================================================================

pub const EMAIL_VERIFICATION_TITLE: &str = "Verify Your Email - Scanopy";

pub const EMAIL_VERIFICATION_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Verify Your Email</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Thanks for signing up for Scanopy! Please verify your email address by clicking the button below:</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{verify_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Verify Email</a>
                        </td>
                    </tr>

                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{verify_url}</p>
                        </td>
                    </tr>

                    <!-- Expiration Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This verification link will expire in 24 hours. If you didn't create a Scanopy account, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;
