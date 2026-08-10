export interface SessionReportOrder {
  readonly adapterGeneration: number;
  readonly revision: bigint;
}

export interface SessionReportSequence extends SessionReportOrder {
  readonly sessionGeneration: number;
}

const MAX_UINT64 = (1n << 64n) - 1n;

/** Allocate immutable producer order without consulting delivery timing. */
export function nextSessionReportSequence(
  previous: SessionReportSequence | undefined,
  adapterGeneration: number,
  sessionGeneration: number,
): SessionReportSequence {
  const sameEpoch =
    previous?.adapterGeneration === adapterGeneration
    && previous.sessionGeneration === sessionGeneration;
  const revision = sameEpoch ? previous.revision + 1n : 1n;
  if (revision > MAX_UINT64) {
    throw new Error("session report revision exhausted uint64");
  }
  return Object.freeze({ adapterGeneration, sessionGeneration, revision });
}
