import { PublicKey } from "@solana/web3.js";
import { SlippageEvaluator, LiquidityHealthCheck } from "./evaluator";

export interface GuardVaultConfig {
  authority: PublicKey;
  tokenMint: PublicKey;
  maxDrawdownBps: number;
  cooldownSlots: number;
  isPaused: boolean;
  lastHaltSlot: number;
}

export class SolanaLiquidityGuardClient {
  private readonly evaluator: SlippageEvaluator;
  private vaultConfig?: GuardVaultConfig;

  constructor(maxDrawdownBps: number = 250) {
    this.evaluator = new SlippageEvaluator(maxDrawdownBps);
  }

  public registerVault(config: GuardVaultConfig): void {
    this.vaultConfig = config;
  }

  public evaluatePoolState(spotPrice: bigint, oracleTwapPrice: bigint, currentSlot: number): LiquidityHealthCheck {
    if (!this.vaultConfig) {
      throw new Error("Vault config is not registered");
    }

    if (this.vaultConfig.isPaused) {
      const elapsed = currentSlot - this.vaultConfig.lastHaltSlot;
      if (elapsed < this.vaultConfig.cooldownSlots) {
        return {
          isHealthy: false,
          deviationBps: 0,
          triggerCircuitBreaker: false,
          reason: `Vault paused: cooldown slot remaining ${this.vaultConfig.cooldownSlots - elapsed}`,
        };
      }
    }

    return this.evaluator.assessDeviation(spotPrice, oracleTwapPrice);
  }
}
