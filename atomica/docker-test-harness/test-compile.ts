#!/usr/bin/env bun

import { compileAndPlaceFramework } from "./dist/index.js";

async function testCompile() {
    try {
        console.log("Testing framework compilation...");
        const outputPath = await compileAndPlaceFramework("./atomica/move-fixtures/head.mrb");
        console.log(`Success! Framework compiled to: ${outputPath}`);
    } catch (error) {
        console.error("Compilation failed:", error);
        process.exit(1);
    }
}

testCompile();
