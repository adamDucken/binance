use reqwest;
use serde::{Deserialize, Serialize};
use std::{error::Error, str::FromStr};
use tokio::time::{sleep, Duration};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

#[derive(Serialize, Deserialize, Debug)]
struct OrderBook {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let symbol = "SUIUSDT";
    let limit = 10;

    loop {
        // Clear screen
        print!("\x1b[2J\x1b[H");

        let url = format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
            symbol, limit
        );

        match reqwest::get(&url).await {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("HTTP Error: {}", response.status());
                } else if let Ok(orderbook) = response.json::<OrderBook>().await {
                    println!("{} Orderbook for {} {} ", RESET, symbol, RESET);

                    // Helper closure to parse and format floats with 4 decimals
                    let fmt_4dec = |s: &String| -> String {
                        match s.parse::<f64>() {
                            Ok(val) => format!("{:.4}", val),
                            Err(_) => s.clone(),
                        }
                    };

                    // Print Bids
                    for bid in &orderbook.bids {
                        let price = fmt_4dec(&bid[0]);
                        let qty = fmt_4dec(&bid[1]);
                        // Print in green
                        println!("{}  {:>8}$  {:>8}{}", GREEN, price, qty, RESET);
                    }

                    // Blank line
                    println!();

                    // Print Asks
                    for ask in &orderbook.asks {
                        let price = fmt_4dec(&ask[0]);
                        let qty = fmt_4dec(&ask[1]);
                        // Print in red
                        println!("{}  {:>8}$ {:>8}{}", RED, price, qty, RESET);
                    }
                } else {
                    eprintln!("Error parsing JSON response.");
                }
            }
            Err(e) => {
                eprintln!("Request error: {}", e);
            }
        }
    }
}
