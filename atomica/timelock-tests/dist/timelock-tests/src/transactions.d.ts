import { AptosClient, AptosAccount } from "aptos";
/**
 * Helper functions for crafting timelock-related transactions
 */
export declare class TimelockTransactions {
    private client;
    private account;
    constructor(client: AptosClient, account: AptosAccount);
    /**
     * Set timelock interval for testing (only works on testnet)
     */
    setIntervalForTesting(intervalMicroseconds: number): Promise<string>;
}
//# sourceMappingURL=transactions.d.ts.map