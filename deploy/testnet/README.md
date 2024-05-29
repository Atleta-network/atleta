Testnet Deployment
==================

We have 10 validators at the moment. 3 of them predefined in the chainspec.
While the others are normally generated accounts.

Additionally we have a boot node.

The session keys for both types of validators configured differently:

- for predefined set of validators the session keys are generated in the chain
  spec, so we insert the keys (`author_insertKeys`) into the keystore of the
  node using `add_session_keys.sh` script;
- for the normal validators, we first generate the keys using
  `author_rotateKeys`, then we add the generated keys using `session.setKeys`
  extrinsic call.

The private keys for the normal accounts should be defined on the CD level and
exported into `config.env` file, which then uploaded to the server. From this
file we get needed information to sign the extrinsics.

The whole content of `config.env` file is:

```
PRIVATE_KEY=<key in hex>
DOCKER_IMAGE=<image name>
BOOTNODE_ADDRESS=<node address in libp2p form>

```

We also expect `chainspec.json` to be uploaded to the same directory, where the
scripts are located.

> WARNING: the normal validators should also be configured on the runtime level
> (i.e. appropriate extrinsics should be called) as they are not predefined in
> the chainspec. Thus the accounts should be prefunded with enough balance to
> become validators. And you need to run a dedicated script to prepare
> everything. Also, the validators should be nominated afterwards.




## Deployment

The workflow is defined in `.github/workflows/deploy-testnet.yml`.

The predefined validators are deployed on a single server. The job is called
`deploy_technical_validators`.

The each of the rest of the validators is deployed on a dedicated server. The
job is called `deploy_validators`.

While the setup of the session keys is automated for the technical
validators, we manually run a script `set_session_keys.js` (`npm run set_keys`) to
add them for the normal validators. Also, we manually run script `validate.js`
(`npm run validate`) to start being a validator. Luckily, it should be done only
once in normal circumstances. And the validators should be updated via `setCode`
in the feature. But anyway, we should improve this workflow and automate what's
currently done manually.
