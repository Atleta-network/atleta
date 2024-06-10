# Metamask pallet

<https://docs.metamask.io/load-content/wallet/how-to/sign-data#use-eth_signtypeddata_v4>  
<https://docs.metamask.io/wallet/reference/eth_signtypeddata_v4/>  
<https://metamask.github.io/eth-sig-util/latest/modules.html>  

## Sign extrinsic call

- Get the current _[ chainId, senderNonce ]_
- Prepare pallet _call_ and [encode as scale][2] then [encode again as cb58][3] it as _callData_
- Sign _message :: { sender, nonce: senderNonce, call: callData }_ -> _signature_
- Send unsigned extrinsic to pallet with _[\_, sender, nonce, signature, call]_

## TODO to complete in mainnet
- Increment Nonce
- Weights Benchmarks
- Share secret (mnemonic) between Metamask and tests to simplify flow
- derive macros for struct (or just macro\_rules! { field: Type as EthAbiType, + })

## Debug using JS

Install required dependencies:
```shell
yarn install @metamask/eth-sig-util
yarn install ethereum-cryptography
yarn install @ethereumjs/util
```

Run it as interactive `node` script (`.load <file.js>`):
```js
const fs = require('fs');
const ethSigUtil = require('@metamask/eth-sig-util');
const { secp256k1 } = await import("ethereum-cryptography/secp256k1.js");
const { Address, ecrecover, bytesToHex, hexToBytes } = await import("@ethereumjs/util");

const privateKey = secp256k1.utils.randomPrivateKey();
const publicKey = secp256k1.getPublicKey(privateKey);

const address = Address.fromPrivateKey(privateKey);
const data = JSON.parse(fs.readFileSync('Payload.json'));
const version = ethSigUtil.SignTypedDataVersion['V4'];

console.log('Domain hash: ', bytesToHex(ethSigUtil.TypedDataUtils.eip712DomainHash(data, version)));
console.log('EIP-712 hash:', bytesToHex(ethSigUtil.TypedDataUtils.eip712Hash(data, version)));

const signature = ethSigUtil.signTypedData({ data, privateKey, version });
const recovered = ethSigUtil.recoverTypedSignature({ data, signature, version });

if (recovered == address.toString()) {
    console.log('recovered address matches origin');
} else {
    console.error('recovered address mismathes origin');
    process.exit(1);
}

console.log('PrivateKey:', bytesToHex(privateKey));
console.log('Address:', address.toString());
console.log('Signature:', signature);
```

Here the example of `Payload.json`:
```json
{
  "types": {
    "EIP712Domain": [
      { "name": "name",              "type": "string"  },
      { "name": "version",           "type": "string"  },
      { "name": "chainId",           "type": "uint256" },
      { "name": "verifyingContract", "type": "address" }
    ],
    "Payload": [
        { "name": "sender", "type": "address" },
        { "name": "nonce",  "type": "uint256" },
        { "name": "call",   "type": "bytes"   }
    ]
  },

  "primaryType": "Payload",
  "domain": {
    "name": "ATLA",
    "version": "1",
    "chainId": 1,
    "verifyingContract": "0x0000000000000000000000000000000000000000"
  },

  "message": {
    "sender": "0xcccccccccccccccccccccccccccccccccccccccc",
    "nonce": "0x1",
    "call": ""
  }
}
```


[1]: https://eips.ethereum.org/EIPS/eip-712 'EIP-712'
[2]: https://docs.substrate.io/reference/scale-codec/ 'SCALE'
[3]: https://support.avax.network/en/articles/4587395-what-is-cb58 'CB58'


<https://metamask.github.io/api-playground/api-documentation/#eth_signTypedData_v4>  
<https://metamask.github.io/test-dapp/#signTypedDataV4>  

Some code examples:
<https://eips.ethereum.org/assets/eip-712/Example.js>  
<https://docs.rs/ethers-derive-eip712/latest/src/ethers_derive_eip712/lib.rs.html#1-172>  

<https://paritytech.github.io/polkadot-sdk/master/src/pallet_sudo/lib.rs.html>  
<https://medium.com/@ashwin.yar/eip-712-structured-data-hashing-and-signing-explained-c8ad00874486>  
<https://ethvigil.com/docs/eip712_sign_example_code/>  
