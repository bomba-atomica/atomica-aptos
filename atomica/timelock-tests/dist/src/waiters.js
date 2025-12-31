/**
 * Helper functions for waiting on timelock events
 */
export class TimelockWaiters {
    queries;
    constructor(queries) {
        this.queries = queries;
    }
    /**
     * Wait for interval rotation
     */
    async waitForIntervalRotation(targetInterval, timeoutSeconds = 120) {
        const startTime = Date.now();
        const timeoutMs = timeoutSeconds * 1000;
        while (Date.now() - startTime < timeoutMs) {
            try {
                const currentInterval = await this.queries.getCurrentInterval();
                if (currentInterval >= targetInterval) {
                    return { current_interval: currentInterval };
                }
            }
            catch (error) {
                // Continue waiting
            }
            await new Promise((resolve) => setTimeout(resolve, 1000));
        }
        throw new Error(`Timeout waiting for interval rotation to ${targetInterval}`);
    }
    /**
     * Wait for public key publication
     */
    async waitForPublicKeyPublication(interval, timeoutSeconds = 60) {
        const startTime = Date.now();
        const timeoutMs = timeoutSeconds * 1000;
        while (Date.now() - startTime < timeoutMs) {
            const transcript = await this.queries.verifyPublicKeyPublished(interval);
            if (transcript) {
                return transcript;
            }
            await new Promise((resolve) => setTimeout(resolve, 1000));
        }
        throw new Error(`Timeout waiting for public key publication for interval ${interval}`);
    }
    /**
     * Wait for secret aggregation
     */
    async waitForSecretAggregation(interval, threshold, timeoutSeconds = 60) {
        const startTime = Date.now();
        const timeoutMs = timeoutSeconds * 1000;
        while (Date.now() - startTime < timeoutMs) {
            const secret = await this.queries.verifySecretAggregated(interval, threshold);
            if (secret) {
                return secret;
            }
            await new Promise((resolve) => setTimeout(resolve, 1000));
        }
        throw new Error(`Timeout waiting for secret aggregation for interval ${interval}`);
    }
}
//# sourceMappingURL=waiters.js.map