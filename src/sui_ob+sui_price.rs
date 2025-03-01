use reqwest;
use serde::{Deserialize, Serialize};
use std::thread;
use std::{error::Error, str::FromStr};
use tokio::time::{sleep, Duration};

// ANSI color codes
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

#[derive(Serialize, Deserialize, Debug)]
struct TickerPrice {
    symbol: String,
    price: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrderBook {
    lastUpdateId: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let symbol = "SUIUSDT";
    let ticker_url = format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}",
        symbol
    );
    let depth_limit = 10;
    let depth_url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
        symbol, depth_limit
    );

    let mut previous_price: Option<f64> = None;

    loop {
        // Fetch ticker price
        let current_price = match reqwest::get(&ticker_url).await {
            Ok(resp) if resp.status().is_success() => match resp.json::<TickerPrice>().await {
                Ok(ticker) => match ticker.price.parse::<f64>() {
                    Ok(price) => Some((ticker.symbol, price)),
                    Err(e) => {
                        eprintln!("Error parsing ticker price: {}", e);
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Error parsing ticker JSON: {}", e);
                    None
                }
            },
            Ok(resp) => {
                eprintln!("Ticker HTTP error: {}", resp.status());
                None
            }
            Err(e) => {
                eprintln!("Ticker request error: {}", e);
                None
            }
        };

        // Print ticker price at the top (with color based on change)
        if let Some((sym, price)) = current_price {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            if let Some(prev) = previous_price {
                let change = price - prev;
                let change_percent = (change / prev) * 100.0;
                let color = if price > prev {
                    GREEN
                } else if price < prev {
                    RED
                } else {
                    RESET
                };
                println!(
                    "[{}] {}: {}{:.6}$ ( {:+.6}$, {:+.2}% ){}",
                    timestamp, sym, color, price, change, change_percent, RESET
                );
            } else {
                println!("[{}] {}: ${:.6}", timestamp, sym, price);
            }
            previous_price = Some(price);
        } else {
            println!("Ticker data unavailable");
        }

        println!(); // Blank line before order book

        // Fetch order book snapshot
        match reqwest::get(&depth_url).await {
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
        // Clear the console (ANSI escape codes)
        print!("\x1b[2J\x1b[H");
    }
}
