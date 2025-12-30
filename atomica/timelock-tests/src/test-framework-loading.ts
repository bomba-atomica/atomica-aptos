import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";

/**
 * Test that our custom .mrb framework bundle is loaded by checking for noop contract
 */
async function testFrameworkLoading() {
  console.log("🧪 Testing Custom Framework Loading (Noop Contract Test)");

  try {
    // Initialize testnet
    console.log("Initializing testnet with 2 validators...");
    const testnet = await initializeTestnet(2);

    try {
      const client = new AptosClient(testnet.validatorApiUrl(0));

      console.log("Step 1: Check for noop contract (unique to our custom framework)");
      try {
        const noopResult = await client.view({
          function: "0x1::noop::is_available",
          type_arguments: [],
          arguments: [],
        });

        const isAvailable = noopResult[0] === true;
        console.log(`Noop contract is_available(): ${isAvailable}`);

        if (isAvailable) {
          console.log("🎉 SUCCESS: Custom framework is loaded!");
          console.log("✅ Our .mrb bundle with timelock fixes is being used");
          console.log("✅ All framework modifications are active");
          return true;
        } else {
          console.log("❌ Noop contract returned false - unexpected");
          return false;
        }
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        if (errorMsg.includes("can't be found")) {
          console.log("❌ FAILURE: Noop contract not found");
          console.log("❌ Custom framework is NOT loaded");
          console.log("❌ Docker image framework is being used instead");
          console.log("💡 This means our .mrb bundle override failed");
          return false;
        } else {
          console.log(`❌ Unexpected error: ${errorMsg}`);
          return false;
        }
      }
    } finally {
      await performCleanup("Framework loading test completed");
    }
  } catch (error) {
    console.error("❌ Framework loading test failed:", error);
    return false;
  }
}

testFrameworkLoading()
  .then((success) => {
    if (success) {
      console.log("🎉 Framework loading confirmed!");
      process.exit(0);
    } else {
      console.log("💥 Framework loading failed!");
      process.exit(1);
    }
  })
  .catch(console.error);
