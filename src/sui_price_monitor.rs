use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct TickerPrice {
    symbol: String,
    price: String,
}

// ANSI color codes
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Binance API endpoint for ticker price
    let url = "https://api.binance.com/api/v3/ticker/price?symbol=SUIUSDT";

    println!("Monitoring SUI/USDT price from Binance...");
    println!("Press Ctrl+C to exit");
    println!("----------------------------------------");

    let mut previous_price: Option<f64> = None;

    loop {
        // Make the HTTP request
        match reqwest::get(url).await {
            Ok(response) => {
                // Check if the request was successful
                if response.status().is_success() {
                    // Parse the JSON response
                    match response.json::<TickerPrice>().await {
                        Ok(ticker) => {
                            // Parse the current price
                            match ticker.price.parse::<f64>() {
                                Ok(current_price) => {
                                    let timestamp =
                                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

                                    // Calculate percentage change if we have a previous price
                                    if let Some(prev_price) = previous_price {
                                        let change = current_price - prev_price;
                                        let change_percent = (change / prev_price) * 100.0;

                                        // Determine color based on price movement
                                        let color = if current_price > prev_price {
                                            GREEN
                                        } else if current_price < prev_price {
                                            RED
                                        } else {
                                            RESET
                                        };

                                        println!(
                                            "[{}] {}: {}{:.6}$ ({:+.6}$, {:+.2}%){}",
                                            timestamp,
                                            ticker.symbol,
                                            color,
                                            current_price,
                                            change,
                                            change_percent,
                                            RESET
                                        );
                                    } else {
                                        // First run, no previous price to compare
                                        println!(
                                            "[{}] {}: ${:.6}",
                                            timestamp, ticker.symbol, current_price
                                        );
                                    }

                                    // Update previous price for next iteration
                                    previous_price = Some(current_price);
                                }
                                Err(e) => println!("Error parsing price: {}", e),
                            }
                        }
                        Err(e) => println!("Error parsing JSON: {}", e),
                    }
                } else {
                    println!(
                        "Error: Failed to fetch price. Status code: {}",
                        response.status()
                    );
                }
            }
            Err(e) => println!("Request error: {}", e),
        }

        //thread::sleep(Duration::from_secs(1));
    }
}
