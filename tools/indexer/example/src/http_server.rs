// http_server.rs

use actix_web::{web, App, Either, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;
use near_indexer;

pub(crate) type BlockCache = Arc<Mutex<HashMap<u64, near_indexer::StreamerMessage>>>;

pub async fn listen_blocks(mut stream: mpsc::Receiver<near_indexer::StreamerMessage>, cache: BlockCache) {
    while let Some(streamer_message) = stream.recv().await {
        let height = streamer_message.block.header.height;

        {
            let mut blocks = cache.lock().unwrap();
            if blocks.len() >= 10000 {
                let oldest_height = *blocks.keys().next().unwrap();
                blocks.remove(&oldest_height);
            }
            blocks.insert(height, streamer_message.clone());
        }

        info!(
            target: "indexer_example",
            "#{} {} Shards: {}, Transactions: {}, Receipts: {}, ExecutionOutcomes: {}",
            height,
            streamer_message.block.header.hash,
            streamer_message.shards.len(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.transactions.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| if let Some(chunk) = &shard.chunk { chunk.receipts.len() } else { 0usize }).sum::<usize>(),
            streamer_message.shards.iter().map(|shard| shard.receipt_execution_outcomes.len()).sum::<usize>(),
        );
    }
}

async fn get_block_by_height(data: web::Data<AppState>, height: web::Path<u64>) -> impl Responder {
    let blocks = data.cache.lock().unwrap();
    if let Some(block) = blocks.get(&height.into_inner()) {
        Either::Left(web::Json(block.clone()))
    } else {
        Either::Right(HttpResponse::NotFound().body("Block not found"))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub cache: BlockCache,
}

pub async fn run_server(cache: BlockCache) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { cache: cache.clone() }))
            .route("/block/{height}", web::get().to(get_block_by_height))
    })
        .bind("127.0.0.1:6526")?
        .run()
        .await
}
