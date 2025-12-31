import { TxnBuilderTypes } from "aptos";
/**
 * Helper functions for crafting timelock-related transactions
 */
export class TimelockTransactions {
    client;
    account;
    constructor(client, account) {
        this.client = client;
        this.account = account;
    }
    /**
     * Set timelock interval for testing (only works on testnet)
     */
    async setIntervalForTesting(intervalMicroseconds) {
        const payload = new TxnBuilderTypes.TransactionPayloadEntryFunction(TxnBuilderTypes.EntryFunction.natural(`${this.account.address()}::timelock_config`, "set_interval_for_testing", [], [TxnBuilderTypes.BCS.bcsSerializeU64(intervalMicroseconds)]));
        const txnRequest = await this.client.generateTransaction(this.account.address(), payload);
        const signedTxn = await this.client.signTransaction(this.account, txnRequest);
        const txnResponse = await this.client.submitTransaction(signedTxn);
        await this.client.waitForTransaction(txnResponse.hash);
        return txnResponse.hash;
    }
}
//# sourceMappingURL=transactions.js.map