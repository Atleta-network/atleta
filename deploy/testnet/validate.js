// start validating (normally should be called once)
// - bond
// - validate

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';

const wsProvider = new WsProvider('ws://127.0.0.1:9944');
const api = await ApiPromise.create({ provider: wsProvider });
const keyring = new Keyring({ type: 'ethereum' });

const dotenv = await import('dotenv');
dotenv.config({ path: './config.env' });

async function main() {
    await api.isReady;

    const validator = keyring.addFromUri(process.env.PRIVATE_KEY);
    const txs = [
        api.tx.staking.bond("75000000000000000000000", 0),
        api.tx.staking.validate(0)
    ];

    const unsub = await api.tx.utility
        .batch(txs)
        .signAndSend(validator, async ({ status }) => {
            if (status.isInBlock) {
                console.log(`included in ${status.asInBlock}`);
                unsub();
                await api.disconnect();
            }
        });
}

main().catch((e) => {
    console.error(e);
    process.exit(1);
});
