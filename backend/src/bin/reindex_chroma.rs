use std::{env, process::ExitCode};

use backend::ai::{chroma_db::ChromaClient, index::reindex_all};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    dotenv::dotenv().ok();
    match run().await {
        Ok((processed, succeeded, failed)) => {
            println!("ONG procesadas: {processed}; indexadas: {succeeded}; fallidas: {failed}");
            if failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(usize, usize, usize), &'static str> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
    let chroma = ChromaClient::from_env().map_err(|_| "Falta CHROMA_URL válida")?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|_| "PostgreSQL no está disponible")?;
    let report = reindex_all(&pool, &chroma).await?;
    pool.close().await;
    Ok((report.processed, report.succeeded, report.failed))
}
