use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use scanopy::server::license::{
    crypto::encoding_key_from_env,
    key::{ConfiguredKey, LicenseKey},
    mint::{license_claims, sign_license},
    types::LicensePlan,
};

#[derive(Parser)]
#[command(name = "scanopy-license")]
#[command(about = "Scanopy license key management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new license key
    Create {
        /// License duration in days (default: 365)
        #[arg(long, default_value = "365")]
        days: u64,
        /// Licensed self-hosted tier. Omit for a legacy/custom key, which
        /// resolves to the unlimited Commercial (self-hosted) plan.
        #[arg(long, value_enum)]
        plan: Option<LicensePlan>,
    },
    /// Verify an existing license key
    Verify {
        /// The license key JWT string
        key: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { days, plan } => {
            let now = Utc::now();
            // `intended_exp` is the user-visible expiry. `exp` is the hard
            // enforcement boundary, 7 days later — a silent grace window.
            let intended_exp = now + Duration::days(days as i64);
            let claims = license_claims(now, intended_exp, None, plan);
            let token = sign_license(&claims, &encoding_key_from_env()?)?;

            println!("{}", token);
            eprintln!(
                "License created. User-visible expiry: {}",
                intended_exp.format("%Y-%m-%d")
            );
            eprintln!(
                "                Hard expiry (with grace): {}",
                chrono::DateTime::from_timestamp(claims.exp, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()
            );

            Ok(())
        }
        Commands::Verify { key } => {
            let key = LicenseKey::new(key);

            // An online key carries no plan or expiry; the cloud supplies them.
            if let ConfiguredKey::Online(claims) = key.key_type() {
                let iat = chrono::DateTime::from_timestamp(claims.iat, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("Status:         VALID (online key)");
                println!("Issued:         {}", iat);
                println!("Org ID:         {}", claims.org_id);
                println!("Key version:    {}", claims.key_version);
                return Ok(());
            }

            let status = key.validate();

            match &status {
                scanopy::server::license::types::LicenseStatus::Valid(claims) => {
                    let exp = chrono::DateTime::from_timestamp(claims.exp, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let intended_exp = chrono::DateTime::from_timestamp(claims.intended_exp, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let iat = chrono::DateTime::from_timestamp(claims.iat, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    println!("Status:         VALID");
                    println!("Issued:         {}", iat);
                    println!("User expiry:    {}", intended_exp);
                    println!("Hard expiry:    {}", exp);
                    if let Some(org_id) = &claims.org_id {
                        println!("Org ID:         {}", org_id);
                    }
                    match &claims.plan {
                        Some(plan) => println!("Plan:           {:?}", plan),
                        None => println!("Plan:           Commercial (legacy/custom)"),
                    }
                }
                scanopy::server::license::types::LicenseStatus::Expired(claims) => {
                    let exp = chrono::DateTime::from_timestamp(claims.exp, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    println!("Status:  EXPIRED");
                    println!("Expired: {}", exp);
                }
                scanopy::server::license::types::LicenseStatus::Invalid(reason) => {
                    println!("Status:  INVALID");
                    println!("Reason:  {}", reason);
                }
                scanopy::server::license::types::LicenseStatus::Pending => {
                    println!("Status:  PENDING");
                }
            }

            Ok(())
        }
    }
}
