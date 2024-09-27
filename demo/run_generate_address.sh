set -e

CLOSING_METHOD="opret1st"
DERIVE_PATH="m/86'/1'/0'/9"
DESC_TYPE="wpkh"
ELECTRUM="blockstream.info:143"
CONSIGNMENT="consignment.rgb"
PSBT="tx.psbt"
IFACE="RGB20Fixed"
conrtact="rgb:WvwsT$tQ-7NAhu$A-IJBzpti-zsh10vn-za4mdZp-i6Ghj!U"

prv="tprv8ZgxMBicQKsPf6A3Li9Yn7Q2X27ytZUncjZFxfjBd3rhhqFrXtzhWsTAswsuYrq52uB2KnqbGWpufieeSvfyYXSw85kgKp4vgfnwiLkWwpf"
program() {
    target/release/brgb $@
}
rgb0() {
    program -n testnet rgb -d data0 -s "$ELECTRUM" $@
}
# program key generate-wallet -d wpkh -x $prv --stock bhlx_stock --wallet bob_wallet
rgb0 state $conrtact $IFACE