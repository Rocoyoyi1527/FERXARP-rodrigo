use std::{env, process::ExitCode};

use backend::admin_bootstrap::{ProvisionConfig, ProvisionError, provision_admin};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    dotenv::dotenv().ok();

    let result = run().await;
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<&'static str, ProvisionError> {
    let config = ProvisionConfig::from_env()?;
    let database_url = env::var("DATABASE_URL").map_err(|_| ProvisionError::MissingDatabaseUrl)?;
    if database_url.trim().is_empty() {
        return Err(ProvisionError::MissingDatabaseUrl);
    }

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|_| ProvisionError::DatabaseUnavailable)?;

    let outcome = provision_admin(&pool, &config).await?;
    pool.close().await;
    Ok(outcome.message())
}
