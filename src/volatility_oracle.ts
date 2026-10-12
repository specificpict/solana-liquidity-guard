export interface VolatilitySample {
  timestamp: number;
  price: bigint;
}

export class VolatilityOracleTracker {
  private samples: VolatilitySample[] = [];
  public readonly sampleWindowMs: number;

  constructor(sampleWindowMs: number = 60_000) {
    this.sampleWindowMs = sampleWindowMs;
  }

  public recordSample(price: bigint, timestamp: number = Date.now()): void {
    this.samples.push({ timestamp, price });
    this.pruneOldSamples(timestamp);
  }

  public calculateRollingTwap(): bigint {
    if (this.samples.length === 0) return 0n;
    const sum = this.samples.reduce((acc, s) => acc + s.price, 0n);
    return sum / BigInt(this.samples.length);
  }

  public getSampleCount(): number {
    return this.samples.length;
  }

  private pruneOldSamples(currentTime: number): void {
    const cutoff = currentTime - this.sampleWindowMs;
    this.samples = this.samples.filter((s) => s.timestamp >= cutoff);
  }
}
