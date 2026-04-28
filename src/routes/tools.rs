use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::AppState;

// ═══════════════════════════════════════════════════════════════
//  PING TOOL
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct PingParams {
    pub host: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PingResponse {
    pub host: String,
    pub output: String,
}

/// GET /api/tools/ping?host=example.com
pub async fn ping(
    Query(params): Query<PingParams>,
) -> Result<Json<PingResponse>, ApiError> {
    let host = params
        .host
        .ok_or_else(|| ApiError::BadRequest("'host' query parameter is required".into()))?;

    // Sanitize: only allow alphanumeric, dots, and dashes
    let sanitized: String = host
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-')
        .collect();

    if sanitized.is_empty() {
        return Err(ApiError::BadRequest("Invalid host".into()));
    }

    let is_windows = cfg!(target_os = "windows");
    let args = if is_windows {
        vec!["-n", "3"]
    } else {
        vec!["-c", "3"]
    };

    let output = tokio::process::Command::new("ping")
        .args(&args)
        .arg(&sanitized)
        .output()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to execute ping: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(Json(PingResponse {
        host: sanitized,
        output: if stdout.is_empty() { stderr } else { stdout },
    }))
}

// ═══════════════════════════════════════════════════════════════
//  ETHERSCAN ANALYZER
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct EtherscanParams {
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtherscanResponse {
    pub address: String,
    pub analysis: AddressAnalysis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_context: Option<TransactionContext>,
}

/// Included when the user provides a tx hash instead of an address.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionContext {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub value_eth: String,
    pub gas_price_gwei: String,
    pub block_number: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressAnalysis {
    pub total_transactions: usize,
    pub unique_addresses_interacted: usize,
    pub average_gas_price_gwei: f64,
    pub first_seen: String,
    pub last_seen: String,
    pub bot_probability: f64,
    pub indicators: Vec<String>,
    pub transactions_sample: Vec<TxSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxSummary {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value_eth: String,
    pub gas_price_gwei: String,
    pub timestamp: String,
}

// Etherscan API response types
#[derive(Debug, Deserialize)]
struct EtherscanApiResponse {
    status: String,
    result: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EtherscanTx {
    hash: String,
    from: String,
    to: String,
    value: String,
    gas_price: String,
    #[serde(rename = "timeStamp")]
    timestamp: String,
}

/// GET /api/tools/etherscan?address=0x...
/// Accepts either a wallet address (42 chars) or a transaction hash (66 chars).
pub async fn etherscan_analyze(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EtherscanParams>,
) -> Result<Json<EtherscanResponse>, ApiError> {
    let input = params
        .address
        .ok_or_else(|| ApiError::BadRequest("'address' query parameter is required".into()))?;

    if !input.starts_with("0x") {
        return Err(ApiError::BadRequest("Input must start with 0x".into()));
    }

    let api_key = state
        .config
        .etherscan_api_key
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest(
            "Etherscan API key not configured. Set ETHERSCAN_API_KEY in .env".into(),
        ))?;

    // Auto-detect: 42 chars = wallet address, 66 chars = tx hash
    let (wallet_address, tx_context) = if input.len() == 66 {
        // Transaction hash — look up the tx to find the sender
        let tx_url = format!(
            "https://api.etherscan.io/api?module=proxy&action=eth_getTransactionByHash&txhash={}&apikey={}",
            input, api_key
        );

        let tx_resp: serde_json::Value = state
            .http_client
            .get(&tx_url)
            .send()
            .await?
            .json()
            .await?;

        let tx_result = tx_resp.get("result")
            .ok_or_else(|| ApiError::BadRequest("Transaction not found".into()))?;

        if tx_result.is_null() {
            return Err(ApiError::NotFound(format!("Transaction {} not found", input)));
        }

        let from = tx_result["from"].as_str().unwrap_or_default().to_string();
        let to = tx_result["to"].as_str().unwrap_or_default().to_string();
        let value_hex = tx_result["value"].as_str().unwrap_or("0x0");
        let gas_price_hex = tx_result["gasPrice"].as_str().unwrap_or("0x0");
        let block_hex = tx_result["blockNumber"].as_str().unwrap_or("0x0");

        let value_wei = u128::from_str_radix(value_hex.trim_start_matches("0x"), 16).unwrap_or(0);
        let gas_wei = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16).unwrap_or(0);
        let block_num = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16).unwrap_or(0);

        let context = TransactionContext {
            tx_hash: input.clone(),
            from: from.clone(),
            to,
            value_eth: format!("{:.6}", value_wei as f64 / 1e18),
            gas_price_gwei: format!("{:.2}", gas_wei as f64 / 1e9),
            block_number: block_num.to_string(),
        };

        if from.is_empty() {
            return Err(ApiError::BadRequest("Could not extract sender from transaction".into()));
        }

        (from, Some(context))
    } else if input.len() == 42 {
        (input, None)
    } else {
        return Err(ApiError::BadRequest(
            "Invalid input: must be a wallet address (42 chars) or transaction hash (66 chars)".into(),
        ));
    };

    // Fetch normal transactions for the wallet
    let url = format!(
        "https://api.etherscan.io/api?module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=desc&apikey={}",
        wallet_address, api_key
    );

    let resp: EtherscanApiResponse = state
        .http_client
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    if resp.status != "1" {
        return Ok(Json(EtherscanResponse {
            address: wallet_address,
            analysis: AddressAnalysis {
                total_transactions: 0,
                unique_addresses_interacted: 0,
                average_gas_price_gwei: 0.0,
                first_seen: "N/A".into(),
                last_seen: "N/A".into(),
                bot_probability: 0.0,
                indicators: vec!["No transactions found or API error".into()],
                transactions_sample: vec![],
            },
            transaction_context: tx_context,
        }));
    }

    let txs: Vec<EtherscanTx> = serde_json::from_value(resp.result)
        .map_err(|e| ApiError::Internal(format!("Failed to parse transactions: {}", e)))?;

    let analysis = analyze_transactions(&wallet_address, &txs);

    Ok(Json(EtherscanResponse {
        address: wallet_address,
        analysis,
        transaction_context: tx_context,
    }))
}

fn analyze_transactions(address: &str, txs: &[EtherscanTx]) -> AddressAnalysis {
    let total = txs.len();

    // Unique addresses interacted with
    let mut unique = std::collections::HashSet::new();
    for tx in txs {
        if tx.from.to_lowercase() == address.to_lowercase() {
            unique.insert(tx.to.to_lowercase());
        } else {
            unique.insert(tx.from.to_lowercase());
        }
    }

    // Average gas price
    let total_gas: f64 = txs
        .iter()
        .filter_map(|tx| tx.gas_price.parse::<f64>().ok())
        .sum();
    let avg_gas_wei = if total > 0 { total_gas / total as f64 } else { 0.0 };
    let avg_gas_gwei = avg_gas_wei / 1_000_000_000.0;

    // Time range
    let timestamps: Vec<i64> = txs
        .iter()
        .filter_map(|tx| tx.timestamp.parse::<i64>().ok())
        .collect();
    let first = timestamps.iter().min().copied().unwrap_or(0);
    let last = timestamps.iter().max().copied().unwrap_or(0);

    // Bot probability heuristics
    let mut indicators = Vec::new();
    let mut bot_score: f64 = 0.0;

    // Indicator 1: High transaction frequency
    if total > 100 {
        let time_span = (last - first).max(1) as f64;
        let tx_per_hour = total as f64 / (time_span / 3600.0);
        if tx_per_hour > 10.0 {
            bot_score += 0.3;
            indicators.push(format!("High frequency: {:.1} tx/hour", tx_per_hour));
        }
    }

    // Indicator 2: Low address diversity
    if total > 10 && unique.len() < total / 5 {
        bot_score += 0.2;
        indicators.push("Low address diversity — interacts with few unique addresses".into());
    }

    // Indicator 3: Uniform gas prices
    let gas_prices: Vec<f64> = txs.iter().filter_map(|tx| tx.gas_price.parse().ok()).collect();
    if gas_prices.len() > 5 {
        let mean = gas_prices.iter().sum::<f64>() / gas_prices.len() as f64;
        let variance = gas_prices.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gas_prices.len() as f64;
        let std_dev = variance.sqrt();
        if mean > 0.0 && std_dev / mean < 0.05 {
            bot_score += 0.25;
            indicators.push("Very uniform gas pricing — likely automated".into());
        }
    }

    // Indicator 4: Round value amounts
    let round_values = txs
        .iter()
        .filter(|tx| {
            tx.value
                .parse::<f64>()
                .map(|v| v > 0.0 && v % 1_000_000_000_000_000_000.0 == 0.0)
                .unwrap_or(false)
        })
        .count();
    if total > 5 && round_values as f64 / total as f64 > 0.8 {
        bot_score += 0.15;
        indicators.push("Mostly round ETH values".into());
    }

    if indicators.is_empty() {
        indicators.push("No strong bot indicators detected".into());
    }

    // Sample transactions
    let sample: Vec<TxSummary> = txs
        .iter()
        .take(10)
        .map(|tx| {
            let value_wei: f64 = tx.value.parse().unwrap_or(0.0);
            let gas_wei: f64 = tx.gas_price.parse().unwrap_or(0.0);
            TxSummary {
                hash: tx.hash.clone(),
                from: tx.from.clone(),
                to: tx.to.clone(),
                value_eth: format!("{:.6}", value_wei / 1e18),
                gas_price_gwei: format!("{:.2}", gas_wei / 1e9),
                timestamp: tx.timestamp.clone(),
            }
        })
        .collect();

    let first_str = chrono::DateTime::from_timestamp(first, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "N/A".into());
    let last_str = chrono::DateTime::from_timestamp(last, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "N/A".into());

    AddressAnalysis {
        total_transactions: total,
        unique_addresses_interacted: unique.len(),
        average_gas_price_gwei: (avg_gas_gwei * 100.0).round() / 100.0,
        first_seen: first_str,
        last_seen: last_str,
        bot_probability: (bot_score.min(1.0) * 100.0).round() / 100.0,
        indicators,
        transactions_sample: sample,
    }
}
