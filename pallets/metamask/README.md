# Metamask pallet

<https://docs.metamask.io/wallet/reference/eth_signtypeddata_v4/>

<https://paritytech.github.io/polkadot-sdk/master/src/pallet_sudo/lib.rs.html>
<https://medium.com/@ashwin.yar/eip-712-structured-data-hashing-and-signing-explained-c8ad00874486>
<https://ethvigil.com/docs/eip712_sign_example_code/>

!!!
<https://eips.ethereum.org/assets/eip-712/Example.js>
<https://docs.rs/ethers-derive-eip712/latest/src/ethers_derive_eip712/lib.rs.html#1-172>

## Sign extrinsic call
<https://docs.metamask.io/load-content/wallet/how-to/sign-data#use-eth_signtypeddata_v4>

- Get the current _chainId_
- Prepare pallet _call_ and [endcode][2] it as _data_
- Sign message with [_sender_, _data_]
- Send unsigned extrinsic to pallet with [TODO: args]

Pallet
- encodes pallet _call_ as [codec][2]()
- encodes these args according to [1] scheme 
- checks that recovered _address_ is the same as _sender_
- ...


[1]: https://eips.ethereum.org/EIPS/eip-712 'EIP-712'
[2]: https://docs.substrate.io/reference/scale-codec/ 'SCALE'

<https://docs.substrate.io/reference/transaction-format/>

<https://github.com/paritytech/txwrapper-core>

<https://metamask.github.io/api-playground/api-documentation/#eth_signTypedData_v4>
<https://metamask.github.io/test-dapp/#signTypedDataV4>
