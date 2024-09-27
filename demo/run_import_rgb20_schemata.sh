set -e

CLOSING_METHOD="opret1st"
DERIVE_PATH="m/86'/1'/0'/9"
DESC_TYPE="wpkh"
ELECTRUM="bihelix-testnet-electrs.iftas.tech:50001"
CONSIGNMENT="consignment.rgb"
PSBT="tx.psbt"
IFACE="RGB20Fixed"
conrtact="rgb:qA5j5J48-8mkf3Rc-XvFuCH6-3xN02e7-di6gFL6-a1jJMjY"

bob_prv="tprv8ZgxMBicQKsPdowaixpUGvzfiMeG6iVZXCnfjj9b8Y2KfsJGBRQSdnobgjtbDWJvoDZZf4jDPJBN5iE7vTTDsRMD7973avpkujyNNrapeHS"
ticker="bhlx"
name="bhlx"
issued_supply=10000
inflation_allowance=10000
method="opret"
program() {
    target/release/brgb $@
}
rgb() {
    program -n testnet rgb -d bhlx_stock -w bob_wallet -p "$bob_prv" -s "$ELECTRUM" $@
}
# rgb address

# rgb issue $ticker $name $issued_supply $inflation_allowance $method
# schema=$(rgb0 schemata | awk '{print $1}')
rgb state $conrtact $IFACE
# echo "schema $schema"