use chrono::Local;
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    error::Error,
    fs,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::join;
use tokio::time::{sleep, Duration};

// Configuration constants - now using a float for more precise intervals
const SYMBOL: &str = "SUIUSDT";
const OUTPUT_DIR: &str = "./orderbook_snapshots";
const DEPTH_LIMIT: u32 = 100;
const UPDATE_INTERVAL: f64 = 0.1; // Seconds (100ms)
const MIN_INTERVAL_BETWEEN_SNAPSHOTS: f64 = 0.1; // Minimum time between snapshots (100ms)

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OrderBook {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PriceData {
    price: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CombinedData {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
    current_price: PriceData,
    local_timestamp: u64,
    local_datetime: String,
}

async fn get_current_price(client: &Client, symbol: &str) -> Result<PriceData, Box<dyn Error>> {
    let url = format!(
        "https://api.binance.us/api/v3/ticker/price?symbol={}",
        symbol
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("API Error getting price: {}", response.status()).into());
    }

    let price_data: serde_json::Value = response.json().await?;
    let price = price_data["price"]
        .as_str()
        .ok_or("Failed to extract price")?
        .to_string();

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

    Ok(PriceData { price, timestamp })
}

async fn get_orderbook_snapshot(
    client: &Client,
    symbol: &str,
    limit: u32,
) -> Result<OrderBook, Box<dyn Error>> {
    let url = format!(
        "https://api.binance.us/api/v3/depth?symbol={}&limit={}",
        symbol, limit
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("API Error getting orderbook: {}", response.status()).into());
    }

    let orderbook: OrderBook = response.json().await?;
    Ok(orderbook)
}

async fn save_snapshot(
    orderbook: &OrderBook,
    price_data: &PriceData,
    symbol: &str,
) -> Result<String, Box<dyn Error>> {
    // Create output directory if it doesn't exist
    if !Path::new(OUTPUT_DIR).exists() {
        fs::create_dir_all(OUTPUT_DIR)?;
    }

    // Format timestamp similar to Python version
    let now = Local::now();
    let timestamp_str = now.format("%Y%m%d_%H%M%S").to_string();
    let datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // Create filename
    let filename = format!("{}/orderbook_{}_{}.json", OUTPUT_DIR, symbol, timestamp_str);

    // Get current timestamp
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Combine data
    let combined_data = CombinedData {
        lastUpdateId: orderbook.lastUpdateId,
        bids: orderbook.bids.clone(),
        asks: orderbook.asks.clone(),
        current_price: PriceData {
            price: price_data.price.clone(),
            timestamp: price_data.timestamp,
        },
        local_timestamp: current_time,
        local_datetime: datetime_str,
    };

    // Serialize and save
    let json_data = serde_json::to_string_pretty(&combined_data)?;
    fs::write(&filename, json_data)?;

    Ok(filename)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting orderbook snapshot capture for {}", SYMBOL);
    println!(
        "Saving snapshots approximately every {:.3}s to {}/",
        UPDATE_INTERVAL, OUTPUT_DIR
    );
    println!(
        "Minimum interval between snapshots: {:.3}s",
        MIN_INTERVAL_BETWEEN_SNAPSHOTS
    );

    // Create a reusable HTTP client
    let client = Arc::new(Client::new());
    let mut last_snapshot_time = Instant::now();

    loop {
        let iteration_start = Instant::now();

        // Check if we should throttle to respect minimum interval
        let time_since_last_snapshot = last_snapshot_time.elapsed().as_secs_f64();
        if time_since_last_snapshot < MIN_INTERVAL_BETWEEN_SNAPSHOTS {
            let sleep_duration =
                Duration::from_secs_f64(MIN_INTERVAL_BETWEEN_SNAPSHOTS - time_since_last_snapshot);
            sleep(sleep_duration).await;
        }

        // Update last snapshot time
        last_snapshot_time = Instant::now();

        // Execute both API calls in parallel
        let client_ref = &client;
        let (orderbook_result, price_result) = join!(
            get_orderbook_snapshot(client_ref, SYMBOL, DEPTH_LIMIT),
            get_current_price(client_ref, SYMBOL)
        );

        match (orderbook_result, price_result) {
            (Ok(snapshot), Ok(price_data)) => {
                match save_snapshot(&snapshot, &price_data, SYMBOL).await {
                    Ok(filename) => {
                        let total_time = iteration_start.elapsed().as_secs_f64();
                        println!("Snapshot saved to {} in {:.3}s", filename, total_time);
                    }
                    Err(e) => eprintln!("Error saving snapshot: {}", e),
                }
            }
            (Err(e), _) => eprintln!("Failed to get orderbook snapshot: {}", e),
            (_, Err(e)) => eprintln!("Failed to get price data: {}", e),
        }

        // Calculate if we need to sleep to maintain the desired interval
        let elapsed = iteration_start.elapsed().as_secs_f64();
        if elapsed < UPDATE_INTERVAL {
            let sleep_duration = Duration::from_secs_f64(UPDATE_INTERVAL - elapsed);
            sleep(sleep_duration).await;
        } else {
            println!("Processing took longer than interval ({:.3}s)", elapsed);
        }
    }
}
