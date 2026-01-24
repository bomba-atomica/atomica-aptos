#!/usr/bin/env bun
/**
 * Docker Testnet Network Test Harness
 *
 * This script tests whether a docker testnet can successfully:
 * 1. Start all validators
 * 2. Reach consensus
 * 3. Produce blocks
 *
 * Usage:
 *   IMAGE_NAME=ghcr.io/bomba-atomica/atomica-aptos/validator:latest tsx test-network.ts
 */

import { spawn } from "child_process";
import { existsSync, writeFileSync, mkdirSync, rmSync } from "fs";
import { resolve } from "path";

const IMAGE_NAME = process.env.IMAGE_NAME || "ghcr.io/bomba-atomica/atomica-aptos/validator:latest";
const NUM_VALIDATORS = 4;
const BASE_API_PORT = 8080;
const BLOCK_PRODUCTION_TIMEOUT_SECS = 120; // 2 minutes to see blocks
const LOG_DIR = process.env.LOG_DIR || "./logs/current";

interface TestResult {
    success: boolean;
    imageName: string;
    timestamp: string;
    validators: ValidatorStatus[];
    blockProgress: {
        initialHeight: number;
        finalHeight: number;
        blocksProduced: number;
        timeElapsed: number;
    };
    errors: string[];
}

interface ValidatorStatus {
    index: number;
    healthy: boolean;
    epoch?: string;
    blockHeight?: string;
    error?: string;
}

interface LedgerInfo {
    chain_id: number;
    epoch: string;
    ledger_version: string;
    block_height: string;
    ledger_timestamp: string;
    node_role: string;
}

function log(message: string): void {
    const timestamp = new Date().toISOString();
    console.log(`[${timestamp}] ${message}`);
}

function execCommand(command: string, args: string[], cwd?: string): Promise<{ stdout: string; stderr: string; code: number }> {
    return new Promise((resolve) => {
        const proc = spawn(command, args, { cwd: cwd || process.cwd(), env: { ...process.env, IMAGE_NAME } });
        let stdout = "";
        let stderr = "";

        proc.stdout?.on("data", (d) => (stdout += d.toString()));
        proc.stderr?.on("data", (d) => (stderr += d.toString()));

        proc.on("close", (code) => {
            resolve({ stdout, stderr, code: code || 0 });
        });
    });
}

async function getLedgerInfo(port: number): Promise<LedgerInfo | null> {
    try {
        const response = await fetch(`http://127.0.0.1:${port}/v1`, {
            signal: AbortSignal.timeout(5000),
        });
        if (response.ok) {
            return await response.json() as LedgerInfo;
        }
        return null;
    } catch (error) {
        return null;
    }
}

async function checkValidatorHealth(index: number): Promise<ValidatorStatus> {
    const port = BASE_API_PORT + index;
    const info = await getLedgerInfo(port);

    if (info) {
        return {
            index,
            healthy: true,
            epoch: info.epoch,
            blockHeight: info.block_height,
        };
    } else {
        return {
            index,
            healthy: false,
            error: "Failed to reach API",
        };
    }
}

async function waitForValidatorsHealthy(timeoutSecs: number): Promise<boolean> {
    const deadline = Date.now() + timeoutSecs * 1000;
    log(`Waiting for ${NUM_VALIDATORS} validators to become healthy (${timeoutSecs}s timeout)...`);

    while (Date.now() < deadline) {
        const statuses = await Promise.all(
            Array.from({ length: NUM_VALIDATORS }, (_, i) => checkValidatorHealth(i))
        );

        const healthyCount = statuses.filter((s) => s.healthy).length;

        if (healthyCount === NUM_VALIDATORS) {
            log(`✓ All ${NUM_VALIDATORS} validators are healthy`);
            return true;
        }

        const statusStr = statuses
            .map((s) => `V${s.index}:${s.healthy ? `epoch${s.epoch},blk${s.blockHeight}` : "ERR"}`)
            .join(" ");
        log(`  Health: ${healthyCount}/${NUM_VALIDATORS} [${statusStr}]`);

        await new Promise((resolve) => setTimeout(resolve, 2000));
    }

    log(`✗ Timeout waiting for validators to become healthy`);
    return false;
}

async function monitorBlockProduction(durationSecs: number): Promise<{ initialHeight: number; finalHeight: number; blocksProduced: number }> {
    log(`Monitoring block production for ${durationSecs} seconds...`);

    const startInfo = await getLedgerInfo(BASE_API_PORT);
    const initialHeight = startInfo ? parseInt(startInfo.block_height, 10) : 0;
    log(`  Initial block height: ${initialHeight}`);

    const startTime = Date.now();
    const deadline = startTime + durationSecs * 1000;

    let lastHeight = initialHeight;
    let lastLogTime = startTime;

    while (Date.now() < deadline) {
        const info = await getLedgerInfo(BASE_API_PORT);
        const currentHeight = info ? parseInt(info.block_height, 10) : 0;

        // Log every 10 seconds or when height changes
        const now = Date.now();
        if (currentHeight !== lastHeight || now - lastLogTime > 10000) {
            log(`  Block height: ${currentHeight} (epoch: ${info?.epoch || "?"}, +${currentHeight - initialHeight} blocks)`);
            lastHeight = currentHeight;
            lastLogTime = now;
        }

        await new Promise((resolve) => setTimeout(resolve, 2000));
    }

    const finalInfo = await getLedgerInfo(BASE_API_PORT);
    const finalHeight = finalInfo ? parseInt(finalInfo.block_height, 10) : 0;
    const blocksProduced = finalHeight - initialHeight;

    log(`  Final block height: ${finalHeight} (${blocksProduced} blocks produced)`);

    return {
        initialHeight,
        finalHeight,
        blocksProduced,
    };
}

async function collectValidatorLogs(): Promise<void> {
    log("Collecting validator logs...");
    mkdirSync(LOG_DIR, { recursive: true });

    for (let i = 0; i < NUM_VALIDATORS; i++) {
        const containerName = `atomica-debug-validator-${i}`;
        const logFile = resolve(LOG_DIR, `validator-${i}.log`);

        log(`  Collecting logs from ${containerName}...`);
        const result = await execCommand("docker", ["logs", containerName]);

        writeFileSync(logFile, `STDOUT:\n${result.stdout}\n\nSTDERR:\n${result.stderr}`);
    }

    log(`✓ Logs saved to ${LOG_DIR}`);
}

async function cleanupTestnet(): Promise<void> {
    log("Cleaning up testnet...");
    await execCommand("docker", ["compose", "down", "-v", "--remove-orphans"]);
    log("✓ Testnet stopped");
}

async function generateGenesis(): Promise<boolean> {
    log("Generating genesis using HOST aptos CLI...");

    // Clean workspace
    const workspaceDir = "./genesis-workspace";
    if (existsSync(workspaceDir)) {
        try {
            rmSync(workspaceDir, { recursive: true, force: true });
        } catch (error: any) {
            if (error.code === 'EACCES') {
                log("  Permission denied, trying with sudo...");
                await execCommand("sudo", ["rm", "-rf", workspaceDir]);
            } else {
                throw error;
            }
        }
    }

    // Run genesis generation using host script
    const result = await execCommand("./generate-genesis-host.sh", [
        NUM_VALIDATORS.toString(),
        "4", // chain_id
        "172.19.0.10", // base_ip
    ]);

    if (result.code !== 0) {
        log(`✗ Genesis generation failed (exit ${result.code})`);
        log(`STDOUT: ${result.stdout}`);
        log(`STDERR: ${result.stderr}`);

        // Save genesis output
        mkdirSync(LOG_DIR, { recursive: true });
        writeFileSync(resolve(LOG_DIR, "genesis-output.log"), `STDOUT:\n${result.stdout}\n\nSTDERR:\n${result.stderr}`);

        return false;
    }

    // Copy artifacts (no sudo needed with host genesis)
    log("Copying genesis artifacts...");
    const artifactsDir = "./genesis-artifacts";
    const validatorsDir = "./validators";

    // Clean up old artifacts
    if (existsSync(artifactsDir)) {
        rmSync(artifactsDir, { recursive: true, force: true });
    }
    if (existsSync(validatorsDir)) {
        rmSync(validatorsDir, { recursive: true, force: true });
    }

    // Copy artifacts
    await execCommand("cp", ["-r", `${workspaceDir}/output`, artifactsDir]);

    mkdirSync(validatorsDir, { recursive: true });

    for (let i = 0; i < NUM_VALIDATORS; i++) {
        await execCommand("cp", ["-r", `${workspaceDir}/validator-${i}`, `${validatorsDir}/validator-${i}`]);
    }

    log("✓ Genesis generation complete");
    return true;
}

async function startTestnet(): Promise<boolean> {
    log("Starting testnet with docker compose...");

    const result = await execCommand("docker", ["compose", "up", "-d"]);

    if (result.code !== 0) {
        log(`✗ Failed to start testnet (exit ${result.code})`);
        log(`STDERR: ${result.stderr}`);
        return false;
    }

    log("✓ Testnet started");
    return true;
}

async function runTest(): Promise<TestResult> {
    const result: TestResult = {
        success: false,
        imageName: IMAGE_NAME,
        timestamp: new Date().toISOString(),
        validators: [],
        blockProgress: {
            initialHeight: 0,
            finalHeight: 0,
            blocksProduced: 0,
            timeElapsed: 0,
        },
        errors: [],
    };

    try {
        // Step 1: Cleanup any existing testnet
        await cleanupTestnet();
        await new Promise((resolve) => setTimeout(resolve, 2000));

        // Step 2: Generate genesis
        const genesisOk = await generateGenesis();
        if (!genesisOk) {
            result.errors.push("Genesis generation failed");
            return result;
        }

        // Step 3: Start testnet
        const startOk = await startTestnet();
        if (!startOk) {
            result.errors.push("Failed to start testnet");
            return result;
        }

        // Step 4: Wait for validators to be healthy
        const healthOk = await waitForValidatorsHealthy(120);
        if (!healthOk) {
            result.errors.push("Validators failed to become healthy");

            // Collect validator status
            result.validators = await Promise.all(
                Array.from({ length: NUM_VALIDATORS }, (_, i) => checkValidatorHealth(i))
            );

            return result;
        }

        // Step 5: Monitor block production
        const startTime = Date.now();
        const blockProgress = await monitorBlockProduction(BLOCK_PRODUCTION_TIMEOUT_SECS);
        const timeElapsed = (Date.now() - startTime) / 1000;

        result.blockProgress = {
            ...blockProgress,
            timeElapsed,
        };

        // Step 6: Check final validator status
        result.validators = await Promise.all(
            Array.from({ length: NUM_VALIDATORS }, (_, i) => checkValidatorHealth(i))
        );

        // Success if blocks were produced
        if (blockProgress.blocksProduced > 0) {
            result.success = true;
            log(`✓ TEST PASSED: Network produced ${blockProgress.blocksProduced} blocks`);
        } else {
            result.errors.push("No blocks produced - network is stuck");
            log(`✗ TEST FAILED: Network produced 0 blocks`);
        }

    } catch (error: any) {
        result.errors.push(`Unexpected error: ${error.message}`);
        log(`✗ TEST FAILED: ${error.message}`);
    } finally {
        // Always collect logs
        await collectValidatorLogs();

        // Save test results
        mkdirSync(LOG_DIR, { recursive: true });
        writeFileSync(
            resolve(LOG_DIR, "test-results.json"),
            JSON.stringify(result, null, 2)
        );
    }

    return result;
}

async function main(): Promise<void> {
    log("=".repeat(80));
    log("Docker Testnet Network Test");
    log(`Image: ${IMAGE_NAME}`);
    log(`Validators: ${NUM_VALIDATORS}`);
    log(`Log directory: ${LOG_DIR}`);
    log("=".repeat(80));

    const result = await runTest();

    log("=".repeat(80));
    log("Test Summary:");
    log(`  Success: ${result.success}`);
    log(`  Image: ${result.imageName}`);
    log(`  Blocks produced: ${result.blockProgress.blocksProduced}`);
    log(`  Healthy validators: ${result.validators.filter((v) => v.healthy).length}/${NUM_VALIDATORS}`);
    if (result.errors.length > 0) {
        log(`  Errors:`);
        result.errors.forEach((err) => log(`    - ${err}`));
    }
    log("=".repeat(80));

    // Exit with appropriate code
    process.exit(result.success ? 0 : 1);
}

main().catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
});
