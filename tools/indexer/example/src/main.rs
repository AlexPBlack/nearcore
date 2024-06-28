use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use actix;
use anyhow::Result;
use clap::Parser;
use configs::{Opts, SubCommand};
use near_indexer;
mod configs;
mod http_server;
use http_server::{listen_blocks, run_server};

async fn main_async() -> Result<()> {
    // We use it to automatically search the for root certificates to perform HTTPS calls
    // (sending telemetry and downloading genesis)
    openssl_probe::init_ssl_cert_env_vars();
    let env_filter = near_o11y::tracing_subscriber::EnvFilter::new(
        "nearcore=info,indexer_example=info,tokio_reactor=info,near=info,\
         stats=info,telemetry=info,indexer=info,near-performance-metrics=info",
    );
    let _subscriber = near_o11y::default_subscriber(env_filter, &Default::default()).global();
    let opts: Opts = Opts::parse();

    let home_dir = opts.home_dir.unwrap_or_else(near_indexer::get_default_home);

    let cache: http_server::BlockCache = Arc::new(Mutex::new(HashMap::new()));

    match opts.subcmd {
        SubCommand::Run => {
            let indexer_config = near_indexer::IndexerConfig {
                home_dir,
                sync_mode: near_indexer::SyncModeEnum::FromInterruption,
                await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::WaitForFullSync,
                validate_genesis: true,
            };

            match near_indexer::Indexer::new(indexer_config) {
                Ok(indexer) => {
                    let stream = indexer.streamer();

                    let cache_clone = cache.clone();
                    actix::spawn(listen_blocks(stream, cache_clone));

                    if let Err(e) = run_server(cache).await {
                        eprintln!("Error running server: {:?}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error initializing indexer: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        SubCommand::Init(config) => {
            if let Err(e) = near_indexer::indexer_init_configs(&home_dir, config.into()) {
                eprintln!("Error initializing config: {:?}", e);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    actix::System::new().block_on(main_async())
}
