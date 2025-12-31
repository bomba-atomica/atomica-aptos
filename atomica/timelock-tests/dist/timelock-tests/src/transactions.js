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
        const payload = {
            type: "entry_function_payload",
            function: "0x1::timelock_config::set_interval_for_testing",
            type_arguments: [],
            arguments: [intervalMicroseconds.toString()],
        };
        const txnRequest = await this.client.generateTransaction(this.account.address(), payload);
        const signedTxn = await this.client.signTransaction(this.account, txnRequest);
        const txnResponse = await this.client.submitTransaction(signedTxn);
        await this.client.waitForTransaction(txnResponse.hash);
        return txnResponse.hash;
    }
}
//# sourceMappingURL=transactions.js.map