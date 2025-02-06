touch state-123.json
echo "" > state-123.json

mkdir -p base/wallet-123
mkdir -p base/masp-123

# cargo run -- --rpc https://rpc.campfire.tududes.com --faucet-sk 00d20e3b3b972b63527069de1f4ea4a6ae47daaaece150d1e3fb0f4ca11eca091d --chain-id campfire-square.ff09671d333707 --id 123 --masp-indexer-url https://masp.campfire.tududes.com/api/v1 $1