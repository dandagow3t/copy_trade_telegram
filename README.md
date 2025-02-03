# Copy Trade Telegram

A Rust application that connects to Telegram, downloads messages from a specified trading group, parses trading signals, and stores them in MongoDB for analysis.

## Features

- **Telegram Integration**: Connects to Telegram using the Grammers client library
- **Message Parsing**: Parses two types of trading messages:
  - Trade Open signals with buy price, market cap, and other metrics
  - Trade Close signals with entry/exit prices and profit percentages
- **MongoDB Storage**: Stores all trades in a normalized format with:
  - Basic trade information (message ID, strategy, token, contract address)
  - Trade-specific data (prices, market metrics, profit/loss)
  - Original message content for reference
- **Indexed Queries**: Supports efficient querying by:
  - Unique message IDs
  - Strategy and token combinations

## Prerequisites

- Rust toolchain
- MongoDB instance running locally (default: mongodb://localhost:27017)
- Telegram API credentials:
  - API ID
  - API Hash

## Environment Setup

Create a `.env` file with your credentials:

```
API_ID=your_api_id
API_HASH=your_api_hash
DB_NAME=your_db_name
DB_URL=your_mongodb_connection_string
```

The MongoDB connection string typically looks like:
- Local instance: `mongodb://localhost:27017`
- Remote instance: `mongodb+srv://username:password@cluster0.example.mongodb.net`

## Usage

Run the program with:

```bash
cargo run -- <name_of_chat>
```

Replace `<name_of_chat>` with the Telegram chat/group name you want to analyze.
