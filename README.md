# Blockchain Sports

The porject is built on a [frontier template](https://github.com/PureStake/frontier) —
[FRAME](https://docs.substrate.io/v3/runtime/frame)-based
[Substrate](https://substrate.io) node with the Ethereum RPC support, ready for
hacking :rocket:




## Build & Run

To build the chain, execute the following commands from the project root:

```
$ cargo build --release
```

To execute the chain, run:

```
$ ./target/release/sportchain-node --dev
```




## Genesis Configuration

In order to view an EVM account, use the `Developer` tab of the Polkadot UI
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

Use the `Developer` app's `RPC calls` tab to query 
`eth > getBalance(address, number)` with Alice's EVM account ID
(`0xd43593c715fdd31c61141abd04a99fd6822c8558`); the value that is returned
should look something like:

```text
x: eth.getBalance
340,282,366,920,938,463,463,374,607,431,768,211,455
```

> Further reading:
> [EVM accounts](https://github.com/danforbes/danforbes/blob/master/writings/eth-dev.md#Accounts)




## Other Prefunded Accounts

Running VPP in development mode will pre-fund several well-known addresses
that (mostly) contain the letters "th" in their names to remind you that they
are for ethereum-compatible usage. These addresses are derived from Substrate's
canonical mnemonic: __bottom drive obey lake curtain smoke basket hold race
lonely fit walk__ followed by the name of an account (i.e `bottom drive obey
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

# Heath:
- Address: 0x931f3600a299fd9B24cEfB3BfF79388D19804BeA
- PrivKey: 0x0d6dcaaef49272a5411896be8ad16c01c35d6f8c18873387b71fbc734759b0ab

# Ida:
- Address: 0xC41C5F1123ECCd5ce233578B2e7ebd5693869d73
- PrivKey: 0x4c42532034540267bf568198ccec4cb822a025da542861fcb146a5fab6433ff8

# Judith:
- Address: 0x2898FE7a42Be376C8BC7AF536A940F7Fd5aDd423
- PrivKey: 0x94c49300a58d576011096bcb006aa06f5a91b34b4383891e8029c21dc39fbb8b
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

- chain ID is __2340__
- chain name is __sportchain__
- currency name is __BCSport Token__
- currency symbol is __BCS__
- currency decimals is __18__
- RPC URL is __http://localhost:9944/__ (or change appropriately to where you
  deploy the node)

## ink! contracts

To generate an initial smart contract, execute the following commands from the project root:

```
$ cargo contract new <conract_name>
```

To compile smart contract, run in the project directory:

```
$ cargo contract build --release
```

## Example 1: ERC20 Contract Deployment using EVM dispatchable

The following steps are also available as a 
[Typescript script](examples/contract-erc20) using Polkadot JS SDK.


### Step 1: Contract creation

The [`truffle`](examples/contract-erc20/truffle) directory contains a
[Truffle](https://www.trufflesuite.com/truffle) project that defines 
[an ERC-20 token](examples/contract-erc20/truffle/contracts/MyToken.sol). 
For convenience, this repository also contains 
[the compiled bytecode of this token contract](examples/contract-erc20/truffle/contracts/MyToken.json#L259), 
which can be used to deploy it to the Substrate blockchain.

> Further reading:
> [the ERC-20 token standard](https://github.com/danforbes/danforbes/blob/master/writings/eth-dev.md#EIP-20-ERC-20-Token-Standard)

Use the Polkadot UI `Extrinsics` app to deploy the contract from Alice's account
(submit the extrinsic as a signed transaction) using `evm > create` with the
following parameters:

```
source: 0xd43593c715fdd31c61141abd04a99fd6822c8558
init: <raw contract bytecode, a very long hex value>
value: 0
gas_limit: 4294967295
gas_price: 1
nonce: <empty> {None}
```

The values for `gas_limit` and `gas_price` were chosen for convenience and have
little inherent or special meaning. Note that `None` for the nonce will
increment the known nonce for the source account, starting from `0x0`, you may
manually set this but will get an "evm.InvalidNonce" error if not set correctly.

Once the extrinsic is in a block, navigate to the `Network` -> `Explorer` tab in
the UI, or open up the browser console to see that the EVM pallet has fired a
`Created` event with an `address` field that provides the address of the
newly-created contract:

```bash
# console:
... {"phase":{"applyExtrinsic":2},"event":{"index":"0x0901","data":["0x8a50db1e0f9452cfd91be8dc004ceb11cb08832f"]} ...

# UI:
evm.Created
A contract has been created at given [address]
   H160: 0x8a50db1e0f9452cfd91be8dc004ceb11cb08832f
```

In this case, however, it is trivial to
[calculate this value](https://ethereum.stackexchange.com/a/46960):
`0x8a50db1e0f9452cfd91be8dc004ceb11cb08832f`. That is because EVM contract
account IDs are determined solely by the ID and nonce of the contract creator's
account and, in this case, both of those values are well-known
(`0xd43593c715fdd31c61141abd04a99fd6822c8558` and `0x0`, respectively).


### Step 2: Check Contract Storage

Use the `Chain State` UI tab to query `evm > accountCodes` for both Alice's and
the contract's account IDs; notice that Alice's account code is empty and the
contract's is equal to the bytecode of the Solidity contract.

The ERC-20 contract that was deployed inherits from
[the OpenZeppelin ERC-20 implementation](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/token/ERC20/ERC20.sol)
and extends its capabilities by adding
[a constructor that mints a maximum amount of tokens to the contract creator](examples/contract-erc20/truffle/contracts/MyToken.sol#L8).
Use the `Chain State` app to query `evm > accountStorage` and view the value
associated with Alice's account in the `_balances` map of the ERC-20 contract;
use the ERC-20 contract address (`0x8a50db1e0f9452cfd91be8dc004ceb11cb08832f`)
as the first parameter and the storage slot to read as the second parameter
(`0x045c0350b9cf0df39c4b40400c965118df2dca5ce0fbcf0de4aafc099aea4a14`). The
value that is returned should be
`0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`.

The storage slot was calculated using
[a provided utility](utils/README.md#--erc20-slot-slot-address). 
(Slot 0 and alice address: `0xd43593c715fdd31c61141abd04a99fd6822c8558`)

> Further reading:
> [EVM layout of state variables in storage](https://solidity.readthedocs.io/en/latest/miscellaneous.html#layout-of-state-variables-in-storage)


### Step 3: Contract Usage

Use the `Developer` -> `Extrinsics` tab to invoke the 
`transfer(address, uint256)` function on the ERC-20 contract with `evm > call` 
and transfer some of the ERC-20 tokens from Alice to Bob.

```text
target: 0x8a50db1e0f9452cfd91be8dc004ceb11cb08832f
source: 0xd43593c715fdd31c61141abd04a99fd6822c8558
input: 0xa9059cbb0000000000000000000000008eaf04151687736326c9fea17e25fc528761369300000000000000000000000000000000000000000000000000000000000000dd
value: 0
gas_limit: 4294967295
gas_price: 1
```

The value of the `input` parameter is an EVM ABI-encoded function call that was
calculated using [the Remix web IDE](http://remix.ethereum.org); it consists of
a function selector (`0xa9059cbb`) and the arguments to be used for the function
invocation. In this case, the arguments correspond to Bob's EVM account ID
(`0x8eaf04151687736326c9fea17e25fc5287613693`) and the number of tokens to be
transferred (`0xdd`, or 221 in hex).

> Further reading:
> [the EVM ABI specification](https://solidity.readthedocs.io/en/latest/abi-spec.html)


### Step 4: Check Bob Contract Storage

After the extrinsic has finalized, use the `Chain State` app to query 
`evm > accountStorage` to see the ERC-20 balances for both Alice and Bob.
