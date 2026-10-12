import { describe, it } from "node:test";
import assert from "node:assert";
import { PublicKey } from "@solana/web3.js";
import { SlippageEvaluator, SolanaLiquidityGuardClient } from "../src/index";

describe("Solana Liquidity Guard Test Suite", () => {
  it("should evaluate healthy liquidity within safe tolerance", () => {
    const evaluator = new SlippageEvaluator(300); // 300 bps (3%)
    const check = evaluator.assessDeviation(10050n, 10000n); // 50 bps deviation
    assert.strictEqual(check.isHealthy, true);
    assert.strictEqual(check.triggerCircuitBreaker, false);
    assert.strictEqual(check.deviationBps, 50);
  });

  it("should trigger circuit breaker when deviation exceeds bps limit", () => {
    const evaluator = new SlippageEvaluator(200); // 200 bps (2%)
    const check = evaluator.assessDeviation(10500n, 10000n); // 500 bps deviation (5%)
    assert.strictEqual(check.isHealthy, false);
    assert.strictEqual(check.triggerCircuitBreaker, true);
    assert.strictEqual(check.deviationBps, 500);
  });

  it("should enforce pause cooldown on guard client", () => {
    const client = new SolanaLiquidityGuardClient(250);
    const mockAuthority = new PublicKey("11111111111111111111111111111111");
    const mockMint = new PublicKey("So11111111111111111111111111111111111111112");

    client.registerVault({
      authority: mockAuthority,
      tokenMint: mockMint,
      maxDrawdownBps: 250,
      cooldownSlots: 200,
      isPaused: true,
      lastHaltSlot: 1000,
    });

    const checkDuringCooldown = client.evaluatePoolState(10000n, 10000n, 1100);
    assert.strictEqual(checkDuringCooldown.isHealthy, false);
    assert.match(checkDuringCooldown.reason || "", /cooldown slot remaining 100/);
  });
});
