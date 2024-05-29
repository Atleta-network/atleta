// this script is expected to be run from the run_validator.sh

import path from 'path';
import { fileURLToPath } from 'url';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { cryptoWaitReady } from '@polkadot/util-crypto';


const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const dotenv = await import('dotenv');
dotenv.config({ path: path.resolve(__dirname, 'config.env') });

async function main() {
    await cryptoWaitReady();

    const wsProvider = new WsProvider('ws://127.0.0.1:9944');
    const api = await ApiPromise.create({ provider: wsProvider });

    const keyring = new Keyring({ type: 'ethereum' });

    const validator = keyring.addFromUri(process.env.PRIVATE_KEY);
    const sessionKeys = await api.rpc.author.rotateKeys();
    const setKeysTx = api.tx.session.setKeys(sessionKeys.toHex(), '');

    // Sign and send the transaction
    const unsub = await setKeysTx.signAndSend(validator, (result) => {
        console.log(`Current status: ${result.status}`);

        if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
            unsub();
        } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            unsub();
        }
    });
    await api.disconnect();
}

main().catch(console.error);
