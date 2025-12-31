import { TimelockQueries } from "./queries";
/**
 * Helper functions for waiting on timelock events
 */
export declare class TimelockWaiters {
    private queries;
    constructor(queries: TimelockQueries);
    /**
     * Wait for interval rotation
     */
    waitForIntervalRotation(targetInterval: number, timeoutSeconds?: number): Promise<{
        current_interval: number;
    }>;
    /**
     * Wait for public key publication
     */
    waitForPublicKeyPublication(interval: number, timeoutSeconds?: number): Promise<Uint8Array>;
    /**
     * Wait for secret aggregation
     */
    waitForSecretAggregation(interval: number, threshold: number, timeoutSeconds?: number): Promise<Uint8Array>;
}
//# sourceMappingURL=waiters.d.ts.map