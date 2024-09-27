set -e

CLOSING_METHOD="opret1st"
DERIVE_PATH="m/86'/1'/0'/9"
DESC_TYPE="wpkh"
ELECTRUM="blockstream.info:143"
CONSIGNMENT="consignment.rgb"
PSBT="tx.psbt"
IFACE="RGB20"

program() {
    target/release/bihelix-rgb-cli $@
}
rgb0() {
    program -n testnet rgb -d data0 -s "$ELECTRUM" $@
}

echo "======== Setup contracts"

rgb0 import demo/rgb-schemata/NonInflatableAssets.rgb
rgb0 import demo/rgb-schemata/NonInflatableAssets-RGB20.rgb

schema=$(rgb0 schemata | awk '{print $1}')

echo "schema $schema"