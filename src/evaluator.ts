export interface LiquidityHealthCheck {
  isHealthy: boolean;
  deviationBps: number;
  triggerCircuitBreaker: boolean;
  reason?: string;
}

export class SlippageEvaluator {
  public readonly maxDeviationBps: number;

  constructor(maxDeviationBps: number = 300) {
    if (maxDeviationBps <= 0 || maxDeviationBps > 5000) {
      throw new Error("Invalid maxDeviationBps: must be between 1 and 5000");
    }
    this.maxDeviationBps = maxDeviationBps;
  }

  public assessDeviation(spotPrice: bigint, oracleTwapPrice: bigint): LiquidityHealthCheck {
    if (oracleTwapPrice <= 0n) {
      throw new Error("Oracle TWAP price must be greater than zero");
    }

    const diff = spotPrice > oracleTwapPrice ? spotPrice - oracleTwapPrice : oracleTwapPrice - spotPrice;
    const deviationBps = Number((diff * 10000n) / oracleTwapPrice);

    if (deviationBps > this.maxDeviationBps) {
      return {
        isHealthy: false,
        deviationBps,
        triggerCircuitBreaker: true,
        reason: `Price deviation ${deviationBps} bps exceeds maximum allowable ${this.maxDeviationBps} bps`,
      };
    }

    return {
      isHealthy: true,
      deviationBps,
      triggerCircuitBreaker: false,
    };
  }
}
