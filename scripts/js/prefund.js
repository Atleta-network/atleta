// prefunds the account with money

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { cryptoWaitReady } from '@polkadot/util-crypto';

async function sendMoney(sudoKey, recipientAddress, amount) {
    await cryptoWaitReady();

    const wsProvider = new WsProvider('wss://testnet-rpc.atleta.network:9944');
    const api = await ApiPromise.create({ provider: wsProvider });
    const keyring = new Keyring({ type: 'ethereum' });
    const sudoAccount = keyring.addFromUri(sudoKey);

    const call = api.tx.balances.forceSetBalance(recipientAddress, amount);
    const sudoCall = api.tx.sudo.sudo(call);

    const unsub = await sudoCall.signAndSend(sudoAccount, async ({ status, events }) => {
        console.log(`Current status: ${status.type}`);

        if (status.isInBlock) {
            console.log(`Transaction included at blockHash ${status.asInBlock}`);
            events.forEach(({ event: { data, method, section }, phase }) => {
                console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
            });
            unsub();
            await api.disconnect();
        } else if (status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${status.asFinalized}`);
            unsub();
            await api.disconnect();
        }
    });

}

async function main() {
    const [, , privateKey, recipientAddress, amount] = process.argv;

    if (!privateKey || !recipientAddress || !amount) {
        console.error('Usage: node add_funds.js <sudoKey> <recipientAddress> <amount>');
        process.exit(1);
    }

    await sendMoney(privateKey, recipientAddress, amount);
}

main().catch(console.error);
