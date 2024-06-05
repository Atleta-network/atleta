# Metamask pallet

<https://docs.metamask.io/load-content/wallet/how-to/sign-data#use-eth_signtypeddata_v4>
<https://docs.metamask.io/wallet/reference/eth_signtypeddata_v4/>
<https://metamask.github.io/eth-sig-util/latest/modules.html>

## Sign extrinsic call

- Get the current _[ chainId, senderNonce ]_
- Prepare pallet _call_ and [endcode][2] it as _callData_
- Sign _message :: { sender, nonce: senderNonce, call: callData }_ -> _signature_
- Send unsigned extrinsic to pallet with _[\_, sender, nonce, signature, call]_


[1]: https://eips.ethereum.org/EIPS/eip-712 'EIP-712'
[2]: https://docs.substrate.io/reference/scale-codec/ 'SCALE'

<https://metamask.github.io/api-playground/api-documentation/#eth_signTypedData_v4>
<https://metamask.github.io/test-dapp/#signTypedDataV4>

!!!
<https://eips.ethereum.org/assets/eip-712/Example.js>
<https://docs.rs/ethers-derive-eip712/latest/src/ethers_derive_eip712/lib.rs.html#1-172>

<https://paritytech.github.io/polkadot-sdk/master/src/pallet_sudo/lib.rs.html>
<https://medium.com/@ashwin.yar/eip-712-structured-data-hashing-and-signing-explained-c8ad00874486>
<https://ethvigil.com/docs/eip712_sign_example_code/>

<https://docs.substrate.io/reference/transaction-format/>
<https://github.com/paritytech/txwrapper-core>

