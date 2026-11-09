// Shared clockx makes time injectable so tests are deterministic.

export interface Clock {
  now(): Date;
}

export const systemClock: Clock = {
  now: () => new Date(),
};

// fixedClock is the test double.
export const fixedClock = (d: Date): Clock => ({
  now: () => new Date(d.getTime()),
});
