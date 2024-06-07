import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';

const wsProvider = new WsProvider('ws://127.0.0.1:9944');
const api = await ApiPromise.create({ provider: wsProvider });
const keyring = new Keyring({ type: 'ethereum' });

const dotenv = await import('dotenv');
dotenv.config({ path: './config.env' });

async function main() {
    await api.isReady;

    const validator = keyring.addFromUri(process.env.PRIVATE_KEY);
    const sessionKeys = await api.rpc.author.rotateKeys();
    const setKeysTx = api.tx.session.setKeys(sessionKeys.toHex(), '');

    // Sign and send the transaction
    const unsub = await setKeysTx.signAndSend(validator, async (result) => {
        console.log(`Current status: ${result.status}`);

        if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            console.log("\n\n\tDone. You have to restart the node!");
            unsub();
            await api.disconnect();
        }
    });
}

main().catch((e) => {
    console.error(e);
    process.exit(1);
});
