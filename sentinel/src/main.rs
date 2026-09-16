#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use sovereign_sentinel::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sovereign_sentinel=debug".into()),
        )
        .json()
        .init();

    let cfg = config::Config::load()?;
    let sentinel = Arc::new(Sentinel::new(cfg)?);

    #[cfg(feature = "dev-http")]
    {
        let s = sentinel.clone();
        tokio::spawn(async move {
            if let Err(e) = dev_http::serve(
                s,
                ([127, 0, 0, 1], 7777).into(),
            )
            .await
            {
                eprintln!("dev-http error: {e}");
            }
        });
        println!("dev-http listening on 127.0.0.1:7777");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    #[cfg(not(feature = "dev-http"))]
    {
        println!("Sentinel ready (Tauri frontend would start here)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
