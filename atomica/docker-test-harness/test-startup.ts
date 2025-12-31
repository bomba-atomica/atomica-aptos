import { initializeTestnet, performCleanup } from "./test/helpers/testnet-lifecycle";

async function testStartup() {
    console.log("Testing docker testnet startup...");
    try {
        const testnet = await initializeTestnet(2);
        console.log("✅ Testnet started successfully");

        console.log("Checking validator status...");
        const info = await testnet.getLedgerInfo(0);
        console.log(`Ledger info: epoch=${info.epoch}, block=${info.block_height}`);

        await performCleanup("Test startup verification completed");
        console.log("✅ Testnet shutdown successfully");
    } catch (error) {
        console.error("❌ Testnet startup failed:", error);
        process.exit(1);
    }
}

testStartup();
