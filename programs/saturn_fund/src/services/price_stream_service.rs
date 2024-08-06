use warp::Filter;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures::{FutureExt, StreamExt};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::unbounded_channel();
    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let fund_pubkey = Pubkey::from_str("fund_account_public_key").unwrap();

    tokio::spawn(async move {
        loop {
            let price_account_pubkey_str = "price_account_public_key_here";
            let price = fetch_fund_price(&client, &fund_pubkey, price_account_pubkey_str).await;
            if let Ok(price) = price {
                if tx.send(price).is_err() {
                    break; // Receiver has dropped, exit the loop
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });

    let price_route = warp::path("price_stream")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let rx = rx.clone();
            ws.on_upgrade(move |websocket| {
                let rx_stream = UnboundedReceiverStream::new(rx);
                rx_stream
                    .map(|price| Ok(warp::ws::Message::text(price.to_string())))
                    .forward(websocket)
                    .map(|result| {
                        if let Err(e) = result {
                            eprintln!("websocket send error: {}", e);
                        }
                    })
            })
        });

    warp::serve(price_route).run(([127, 0, 0, 1], 3030)).await;
}

async fn fetch_fund_price(client: &RpcClient, fund_pubkey: &Pubkey, price_account_pubkey_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    // Convert the string pubkey to a Pubkey type
    let price_account_pubkey = Pubkey::from_str(price_account_pubkey_str)?;

    // Fetch the account data using the dynamically provided public key
    let price_account_data = client.get_account_data(&price_account_pubkey)?;

    // Decode the price from the account data
    let price = decode_price_from_data(&price_account_data)?;

    Ok(price)
}

fn decode_price_from_data(data: &[u8]) -> Result<u64, Box<dyn std::error::Error>> {
   
    if data.len() >= 8 {
        let price = u64::from_le_bytes(data[0..8].try_into().unwrap());
        Ok(price)
    } else {
        Err("Invalid data length for price information".into())
    }
}