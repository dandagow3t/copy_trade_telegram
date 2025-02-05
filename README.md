# Copy Trade Telegram Bot

A Rust-based Telegram bot that monitors trading signals from specified groups and can automatically execute trades on Solana.

## Features

- Monitor Telegram groups for trading signals
- Parse and store trade signals in MongoDB
- Execute trades automatically on Solana
- Support for various DEXes including Jupiter and Pump.fun
- Configurable trade parameters including position size and slippage
- Robust error handling and logging

## Prerequisites

- Rust toolchain
- MongoDB instance
- Telegram API credentials
- Solana wallet and RPC endpoint

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/copy-trade-telegram
cd copy-trade-telegram
```

2. Copy the example environment file and fill in your credentials:
```bash
cp .env_example .env
```

3. Build the project:
```bash
cargo build --release
```

## Configuration

Create a `.env` file with the following parameters:

```env
# Telegram Configuration
TG_ID=                    # Your Telegram API ID
TG_HASH=                  # Your Telegram API Hash
TG_POOL_FREQUENCY=2       # How often to check for new messages (in seconds)
GROUP_NAME=               # Target Telegram group name

# Database Configuration
DB_NAME=                  # MongoDB database name
MONGODB_URI=mongodb://localhost:27017

# Solana Configuration
SOLANA_RPC_URL=          # Solana RPC endpoint
SOLANA_PRIVATE_KEY=      # Your wallet's private key in base58 format

# Trading Configuration
TRADE_ON=true            # Enable/disable automatic trading
POSITION_SIZE_SOL=0.005  # Position size in SOL
SLIPPAGE_BPS=500        # Slippage tolerance in basis points (500 = 5%)
```

## Usage

Run the bot:
```bash
cargo run --release
```

## Features

### Telegram Integration
- Connects to specified Telegram groups
- Monitors and parses trading signals
- Stores trade information in MongoDB

### Trading Capabilities
- Automatic trade execution on Solana
- Support for multiple DEXes:
  - Jupiter Protocol
  - Pump.fun
- Configurable position sizes and slippage
- Support for both market buys and sells

### Solana Integration
- Native Solana transaction handling
- Support for SPL tokens
- Automatic ATA (Associated Token Account) creation
- Priority fee management
- Transaction retry mechanism

### Database
- MongoDB integration for trade storage
- Indexed collections for efficient querying
- Trade history tracking

## Development

### Project Structure
- `src/tg_copy/` - Telegram integration and signal parsing
- `src/solana/` - Solana blockchain interaction
- `src/signer/` - Transaction signing implementations
- `src/config/` - Configuration management
- `src/common/` - Shared utilities

### Testing

Run the test suite:
```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Disclaimer

This software is for educational purposes only. Use at your own risk. The developers are not responsible for any financial losses incurred through the use of this software.