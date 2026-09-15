use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use jsonwebtoken::{Algorithm, Header};
use scanopy::server::license::{
    crypto::encoding_key_from_env,
    key::{LicenseKey, LicenseKeyType},
    types::{LicenseClaims, LicensePlan},
};

/// Silent grace window added past the user-visible expiry. Hard-coded —
/// per-tier grace is explicitly out of scope.
const GRACE_PERIOD_DAYS: i64 = 7;

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
            let exp = intended_exp + Duration::days(GRACE_PERIOD_DAYS);

            let claims = LicenseClaims {
                sub: "scanopy-license".to_string(),
                iss: "scanopy".to_string(),
                iat: now.timestamp(),
                exp: exp.timestamp(),
                intended_exp: intended_exp.timestamp(),
                org_id: None,
                plan,
            };

            let header = Header::new(Algorithm::EdDSA);
            let key = encoding_key_from_env()?;
            let token = jsonwebtoken::encode(&header, &claims, &key)?;

            println!("{}", token);
            eprintln!(
                "License created. User-visible expiry: {}",
                intended_exp.format("%Y-%m-%d")
            );
            eprintln!(
                "                Hard expiry (with grace): {}",
                exp.format("%Y-%m-%d")
            );

            Ok(())
        }
        Commands::Verify { key } => {
            let key = LicenseKey::new(key);

            // An online key carries no plan or expiry; the cloud supplies them.
            if let LicenseKeyType::Online(claims) = key.key_type() {
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
