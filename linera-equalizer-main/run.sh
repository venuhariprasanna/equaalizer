#!/bin/bash

# Get the number of proxies and servers from command line arguments or use default values.
NUM_VALIDATORS=${1:-1}
SHARDS_PER_VALIDATOR=${2:-4}

cd ../linera-protocol/scripts/

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
CONF_DIR="${SCRIPT_DIR}/../configuration"

cd $SCRIPT_DIR/..

# For debug builds:
cargo build && cd target/debug
# For release builds:
# cargo build --release && cd target/release

# Clean up data files
rm -rf *.json *.txt *.db

# Make sure to clean up child processes on exit.
trap 'kill $(jobs -p)' EXIT

set -x

# Create configuration files for NUM_VALIDATORS validators with SHARDS_PER_VALIDATOR shards each.
# * Private server states are stored in `server*.json`.
# * `committee.json` is the public description of the Linera committee.
VALIDATOR_FILES=()
for i in $(seq 1 $NUM_VALIDATORS); do
    VALIDATOR_FILES+=("$CONF_DIR/validator_$i.toml")
done
./linera-server generate --validators "${VALIDATOR_FILES[@]}" --committee committee.json

# Create configuration files for 10 user chains.
# * Private chain states are stored in one local wallet `wallet.json`.
# * `genesis.json` will contain the initial balances of chains as well as the initial committee.

./linera --wallet wallet.json --storage rocksdb:linera.db create-genesis-config 10 --genesis genesis.json --initial-funding 10 --committee committee.json

# Initialize the second wallet.
./linera --wallet wallet_2.json --storage rocksdb:linera_2.db wallet init --genesis genesis.json

# Start servers and create initial chains in DB
for I in $(seq 1 $NUM_VALIDATORS)
do
    ./linera-proxy server_"$I".json &

    for J in $(seq 0 $((SHARDS_PER_VALIDATOR - 1)))
    do
        ./linera-server run --storage rocksdb:server_"$I"_"$J".db --server server_"$I".json --shard "$J" --genesis genesis.json &
    done
done

sleep 3;

# Create second wallet with unassigned key.
KEY=$(./linera --wallet wallet_2.json --storage rocksdb:linera_2.db keygen)

# Open chain on behalf of wallet 2.
EFFECT_AND_CHAIN=$(./linera --wallet wallet.json --storage rocksdb:linera.db open-chain --to-public-key "$KEY")
EFFECT=$(echo "$EFFECT_AND_CHAIN" | sed -n '1 p')

# Assign newly created chain to unassigned key.
./linera --wallet wallet_2.json --storage rocksdb:linera_2.db assign --key "$KEY" --message-id "$EFFECT"

processwalletshow () {
    if [[ $1 -eq 1 ]] 
	then
		SHOW=$(./linera --wallet wallet.json --storage rocksdb:linera.db wallet show)
	elif [[ $1 -eq 2 ]] 
	then
		SHOW=$(./linera --wallet wallet_2.json --storage rocksdb:linera_2.db wallet show)
	fi
	if [[ "$3" = "chain" ]]
	then
	    RESULT=${SHOW:$((470+936*$2)):64}
	elif [[ "$3" = "owner" ]]
	then
	    RESULT=${SHOW:$((713+936*$2)):64}
    fi
}

processwalletshow 1 9 chain
WALLET_ONE_DEFAULT_CHAIN=$RESULT
processwalletshow 1 9 owner
WALLET_ONE_DEFAULT_OWNER=$RESULT
processwalletshow 1 8 chain
WALLET_ONE_ANOTHER_CHAIN=$RESULT
processwalletshow 1 8 owner
WALLET_ONE_ANOTHER_OWNER=$RESULT
processwalletshow 2 0 chain
WALLET_TWO_DEFAULT_CHAIN=$RESULT
processwalletshow 2 0 owner
WALLET_TWO_DEFAULT_OWNER=$RESULT

LINERA_WALLET=$(realpath wallet.json)
LINERA_STORAGE=rocksdb:$(dirname "$LINERA_WALLET")/linera.db
LINERA_WALLET_2=$(realpath wallet_2.json)
LINERA_STORAGE_2=rocksdb:$(dirname "$LINERA_WALLET_2")/linera_2.db

cd ../../../linera_logger


LOGGER_BYTECODE_ID=$(linera --wallet "$LINERA_WALLET" --storage "$LINERA_STORAGE" publish-bytecode logger/target/wasm32-unknown-unknown/release/logger_{contract,service}.wasm)
sleep 30
LOGGER_APPLICATION_ID=$(linera --wallet "$LINERA_WALLET" --storage "$LINERA_STORAGE" create-application "$LOGGER_BYTECODE_ID")
sleep 10

cd ../equalizer

EQUALIZER_BYTECODE_ID=$(linera --wallet "$LINERA_WALLET" --storage "$LINERA_STORAGE" publish-bytecode aqueduct/target/wasm32-unknown-unknown/release/aqueduct_{contract,service}.wasm)
sleep 30
EQUALIZER_APPLICATION_ID=$(linera --wallet "$LINERA_WALLET" --storage "$LINERA_STORAGE" create-application "$EQUALIZER_BYTECODE_ID" --json-parameters "{\"logger_application_id\":\"${LOGGER_APPLICATION_ID}\"}" --required-application-ids "$LOGGER_APPLICATION_ID")

echo "open three consoles: run a service for each wallet and start the frontend; press enter to proceed"

read

PORT_1=8080
PORT_2=8081

xdg-open "http://localhost:3000/$EQUALIZER_APPLICATION_ID?port=$PORT_1"
xdg-open "http://localhost:3000/$EQUALIZER_APPLICATION_ID?port=$PORT_2"

read
