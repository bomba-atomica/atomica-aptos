import { AptosClient } from "aptos";
/**
 * Helper functions for querying timelock blockchain state
 */
export declare class TimelockQueries {
    private client;
    constructor(client: AptosClient);
    /**
     * Get current timelock interval
     */
    getCurrentInterval(): Promise<number>;
    /**
     * Check if timelock is initialized
     */
    isTimelockInitialized(): Promise<boolean>;
    /**
     * Verify public key published for interval
     */
    verifyPublicKeyPublished(interval: number): Promise<Uint8Array | null>;
    /**
     * Verify secret aggregated for interval
     */
    verifySecretAggregated(interval: number, threshold: number): Promise<Uint8Array | null>;
}
//# sourceMappingURL=queries.d.ts.map