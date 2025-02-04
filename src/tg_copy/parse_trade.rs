use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    StopLoss,
    TakeProfit,
    Manual,
}

impl FromStr for OperationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SL" => Ok(OperationType::StopLoss),
            "TP" => Ok(OperationType::TakeProfit),
            "Manual" => Ok(OperationType::Manual),
            _ => Err(format!("Unknown operation type: {}", s)),
        }
    }
}

impl ToString for OperationType {
    fn to_string(&self) -> String {
        match self {
            OperationType::StopLoss => "SL".to_string(),
            OperationType::TakeProfit => "TP".to_string(),
            OperationType::Manual => "Manual".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeClose {
    pub strategy: String,
    pub op_type: OperationType,
    pub token: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub profit_pct: f64,
    pub contract_address: String,
}

#[derive(Debug, Clone)]
pub struct TradeOpen {
    pub strategy: String,
    pub token: String,
    pub buy_price: f64,
    pub num_buys: u32,
    pub total_buys: Option<f64>,
    pub time_window: u32,
    pub contract_address: String,
    pub market_cap: f64,
}

#[derive(Debug, Clone)]
pub enum Trade {
    Open(TradeOpen),
    Close(TradeClose),
}

fn extract_contract_address(text: &str) -> Option<String> {
    if let Some(ca_line) = text.lines().find(|line| line.contains("CA:")) {
        ca_line
            .split("CA:")
            .nth(1)
            .map(|s| s.trim().replace("pump", "").trim().to_string())
    } else {
        None
    }
}

fn clean_price_string(text: &str) -> Option<&str> {
    text.find('$').map(|i| &text[i..])
}

fn parse_price(text: &str) -> Option<f64> {
    clean_price_string(text)?
        .trim_start_matches('$')
        .parse::<f64>()
        .ok()
}

fn extract_strategy(lines: &[&str]) -> Option<String> {
    // For trade close messages, strategy is on the second line
    if let Some(second_line) = lines.get(1) {
        if !second_line.contains('|') {
            return Some(second_line.trim().to_string());
        }
    }

    // For trade open messages, strategy is after '|' in the MC line
    lines
        .iter()
        .find(|line| line.contains("MC:"))
        .and_then(|line| line.split('|').nth(1))
        .map(|s| s.trim().to_string())
}

pub fn parse_trade(message: &str) -> Option<Trade> {
    parse_trade_close(message)
        .map(Trade::Close)
        .or_else(|| parse_trade_open(message).map(Trade::Open))
}

pub fn parse_trade_close(message: &str) -> Option<TradeClose> {
    let lines: Vec<&str> = message.lines().collect();

    // First line should contain token name and operation type
    let first_line = lines.first()?;

    // Extract strategy from second line
    let strategy = extract_strategy(&lines)?;

    // Extract operation type and token
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let token = parts[1].to_string();
    let op_type = match parts[2] {
        "SL" => OperationType::StopLoss,
        "TP" => OperationType::TakeProfit,
        "Manual" => OperationType::Manual,
        _ => return None,
    };

    // Find the price line that contains →
    let price_line = lines.iter().find(|line| line.contains("→"))?.trim();

    // Parse prices
    let price_parts: Vec<&str> = price_line.split("→").collect();
    if price_parts.len() != 2 {
        return None;
    }

    let entry_price = parse_price(price_parts[0].trim())?;

    // Extract exit price and profit percentage
    let exit_price_parts: Vec<&str> = price_parts[1].split('(').collect();
    if exit_price_parts.is_empty() {
        return None;
    }

    let exit_price = parse_price(exit_price_parts[0].trim())?;

    // Parse profit percentage
    let profit_str = exit_price_parts
        .get(1)?
        .trim_end_matches(')')
        .trim_end_matches('%');

    let profit_pct = profit_str.parse::<f64>().ok()?;

    let contract_address = extract_contract_address(message)?;

    Some(TradeClose {
        strategy,
        op_type,
        token,
        entry_price,
        exit_price,
        profit_pct,
        contract_address,
    })
}

fn parse_market_cap(text: &str) -> Option<f64> {
    let mc_str = text
        .trim_start_matches("MC: $")
        .trim_end_matches('k')
        .trim_end_matches('M');

    let base_value = mc_str.parse::<f64>().ok()?;

    if text.contains('k') {
        Some(base_value * 1000.0)
    } else if text.contains('M') {
        Some(base_value * 1_000_000.0)
    } else {
        Some(base_value)
    }
}

pub fn parse_trade_open(message: &str) -> Option<TradeOpen> {
    let lines: Vec<&str> = message.lines().collect();

    // Extract strategy from second line
    let strategy = extract_strategy(&lines)?;

    // Extract token from first line
    let first_line = lines.first()?;
    let token = if first_line.contains("→") {
        first_line.split("→").nth(1)?.trim().to_string()
    } else {
        first_line.split_whitespace().nth(3)?.to_string()
    };
    let mc_line = lines.iter().find(|line| line.contains("MC:"))?;
    let market_cap = parse_market_cap(mc_line.split('|').next()?.trim())?;

    // Find buy price line
    let buy_price_line = lines.iter().find(|line| line.contains("Buy Price:"))?;

    let buy_price = parse_price(buy_price_line.split("Buy Price:").nth(1)?.trim())?;

    // Extract number of buys and time window
    let buys_line = lines
        .iter()
        .find(|line| line.contains("buys") || line.contains("buyers"))?;

    let num_buys = buys_line
        .split_whitespace()
        .find(|s| s.parse::<u32>().is_ok())?
        .parse::<u32>()
        .ok()?;

    // Extract total buys if available
    let total_buys = buys_line
        .split(',')
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<f64>().ok());

    // Extract time window
    let time_window = buys_line
        .split('(')
        .nth(1)?
        .split('s')
        .next()?
        .parse::<u32>()
        .ok()?;

    let contract_address = extract_contract_address(message)?;

    Some(TradeOpen {
        strategy,
        token,
        buy_price,
        num_buys,
        total_buys,
        time_window,
        contract_address,
        market_cap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_trade_close_sl() {
        let message = "🔴 ABYS SL\npasdfddddsec\n└ $0.001122 → $0.000165 (-85.3%)\n└─ CA: HXFuUcBQkcfUNksDkgxBVapg3coA4UsSxe6ny9WFpump";

        let trade = parse_trade_close(message).unwrap();

        assert_eq!(trade.op_type, OperationType::StopLoss);
        assert_eq!(trade.strategy, "pasdfddddsec");
        assert_eq!(trade.token, "ABYS");
        assert_eq!(trade.entry_price, 0.001122);
        assert_eq!(trade.exit_price, 0.000165);
        assert_eq!(trade.profit_pct, -85.3);
        assert_eq!(
            trade.contract_address,
            "HXFuUcBQkcfUNksDkgxBVapg3coA4UsSxe6ny9WF"
        );
    }

    #[test]
    fn test_parse_trade_close_tp() {
        let message = "🔴 ABYS TP\nprereeeet\n└ $0.000583 → $0.001169 (+100.7%)\n└─ CA: HXFuUcBQkcfUNksDkgxBVapg3coA4UsSxe6ny9WFpump";

        let trade = parse_trade_close(message).unwrap();

        assert_eq!(trade.op_type, OperationType::TakeProfit);
        assert_eq!(trade.strategy, "prereeeet");
        assert_eq!(trade.token, "ABYS");
        assert_eq!(trade.entry_price, 0.000583);
        assert_eq!(trade.exit_price, 0.001169);
        assert_eq!(trade.profit_pct, 100.7);
        assert_eq!(
            trade.contract_address,
            "HXFuUcBQkcfUNksDkgxBVapg3coA4UsSxe6ny9WF"
        );
    }
}
