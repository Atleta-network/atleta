# Atleta Network

The project is built on a [frontier template](https://github.com/polkadot-evm/frontier) —
[FRAME](https://docs.substrate.io/reference/)-based [Substrate](https://substrate.io) node
with the [Ethereum RPC](https://ethereum.org/en/developers/docs/apis/json-rpc/#json-rpc-methods) support.

## Build the node

To build the network node execute the following command:

```shell
cargo build --release
```

To execute the _devnet_ validator, run:

```shell
./target/release/atleta-node --dev
```

### Build features

To build node with features execute the following:

```shell
cargo build --release --features <features_list>
```

where `<features_list>` is space or comma separated list of features listed below.

#### `devnet-runtime` | `testnet-runtime`

This feature allows to shorten time intervals for certain runtime periods, such as bonding, voting for example.
Useful when running a _devnet_ or _testnet_, as it allows some logic to run more frequently.

## Genesis Configuration

In order to view an EVM account, use the [`Developer`](https://polkadot.js.org/apps/#/settings/developer) tab of the Polkadot UI
`Settings` app to define the EVM `Account` type as below:

```json
{
  "AccountId": "EthereumAccountId",
  "Address": "AccountId",
  "Balance": "u128",
  "RefCount": "u8",
  "LookupSource": "AccountId",
  "Account": {
    "nonce": "U256",
    "balance": "u128"
  },
  "EthTransaction": "LegacyTransaction",
  "DispatchErrorModule": "DispatchErrorModuleU8",
  "EthereumSignature": {
    "r": "H256",
    "s": "H256",
    "v": "U8"
  },
  "ExtrinsicSignature": "EthereumSignature",
  "TxPoolResultContent": {
    "pending": "HashMap<H160, HashMap<U256, PoolTransaction>>",
    "queued": "HashMap<H160, HashMap<U256, PoolTransaction>>"
  },
  "TxPoolResultInspect": {
    "pending": "HashMap<H160, HashMap<U256, Summary>>",
    "queued": "HashMap<H160, HashMap<U256, Summary>>"
  },
  "TxPoolResultStatus": {
    "pending": "U256",
    "queued": "U256"
  },
  "Summary": "Bytes",
  "PoolTransaction": {
    "hash": "H256",
    "nonce": "U256",
    "blockHash": "Option<H256>",
    "blockNumber": "Option<U256>",
    "from": "H160",
    "to": "Option<H160>",
    "value": "U256",
    "gasPrice": "U256",
    "gas": "U256",
    "input": "Bytes"
  }
}
```

Use the [`Developer`](https://polkadot.js.org/apps/#/rpc) app's `RPC calls` tab to query
`eth > getBalance(address, number)` with Alith's EVM account ID
(`0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac`); the value that is returned
should look something like:

```text
x: eth.getBalance
75,000,000,000,000,000,000,000,000
```

> Further reading:
> [EVM accounts](https://github.com/danforbes/danforbes/blob/master/writings/eth-dev.md#Accounts)

## Other Prefunded Accounts

Running VPP in development mode will pre-fund several well-known addresses
that (mostly) contain the letters "th" in their names to remind you that they
are for ethereum-compatible usage. These addresses are derived from Substrate's
canonical mnemonic: **bottom drive obey lake curtain smoke basket hold race
lonely fit walk** followed by the name of an account (i.e `bottom drive obey
lake curtain smoke basket hold race lonely fit walk//Alith`)

```
# Alith (sudo):
- Address: 0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac
- PrivKey: 0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133

# Baltathar:
- Address: 0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0
- PrivKey: 0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b

# Charleth:
- Address: 0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc
- PrivKey: 0x0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b

# Dorothy:
- Address: 0x773539d4Ac0e786233D90A233654ccEE26a613D9
- PrivKey: 0x39539ab1876910bbf3a223d84a29e28f1cb4e2e456503e7e91ed39b2e7223d68

# Ethan:
- Address: 0xFf64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB
- PrivKey: 0x7dce9bc8babb68fec1409be38c8e1a52650206a7ed90ff956ae8a6d15eeaaef4

# Faith:
- Address: 0xC0F0f4ab324C46e55D02D0033343B4Be8A55532d
- PrivKey: 0xb9d2ea9a615f3165812e8d44de0d24da9bbd164b65c4f0573e1ce2c8dbd9c8df

# Goliath:
- Address: 0x7BF369283338E12C90514468aa3868A551AB2929
- PrivKey: 0x96b8a38e12e1a31dee1eab2fffdf9d9990045f5b37e44d8cc27766ef294acf18
```

Also, the pre-funded default account for testing purposes is:

```
# Gerald:
- Address: 0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b
- PrivKey: 0x99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342
```

## Configuring Ethereum Wallet For Development

The node should run locally in `--dev` mode.

Then you need to configure the network this way:

- chain ID is **2340**
- chain name is **atleta**
- currency name is **Atleta Token**
- currency symbol is **ATLA**
- currency decimals is **18**
- RPC URL is **http://localhost:9944/** (or change appropriately to where you deploy the node)
