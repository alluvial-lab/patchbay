import type { TokenCommuneGatewaySnapshot } from "../../src/snapshot_projection.js";

const noTelemetryContribution = {
  provider: "anthropic",
  declaredShare: 0.2,
  health: { state: "exhausted", exhaustedUntil: "2026-08-08T00:00:00.000Z" },
  capacity: [],
  fingerprint: { state: "held", templateSource: "compiled", since: "2026-08-07T10:00:00.000Z", diffPresent: true },
} as const;

export const allSourcesGateway: TokenCommuneGatewaySnapshot = {
  status: {
    status: "reported",
    value: {
      ok: false,
      anthropicHealth: { state: "auth_broken", reason: "upstream credential expired" },
      contributions: [
        {
          contributionId: "status-anthropic",
          provider: "anthropic",
          readings: [{
            window: "parallel", usedFraction: null, usedUnits: 2, limitUnits: null,
            resetsAt: null, source: "declared", observedAt: "2026-08-07T11:59:00.000Z",
          }],
        },
        { contributionId: "status-only-row", provider: "status-only", readings: [] },
      ],
    },
  },
  pool: {
    status: "reported",
    value: {
      contributions: [
        noTelemetryContribution,
        {
          provider: "openai-codex",
          declaredShare: 0.4,
          health: { state: "fresh" },
          capacity: [{
            window: "5h", usedFraction: 0.5, usedUnits: null, limitUnits: 100,
            resetsAt: "2026-08-07T17:00:00.000Z", source: "headers", observedAt: "2026-08-07T11:58:00.000Z",
          }],
          fingerprint: { state: "ok", templateSource: "override", since: null, diffPresent: false },
        },
        {
          provider: "anthropic",
          declaredShare: 0.3,
          health: { state: "auth_broken", reason: "revoked key" },
          capacity: [{
            window: "7d", usedFraction: 0, usedUnits: 0, limitUnits: null,
            resetsAt: null, source: "usage_endpoint", observedAt: "2026-08-07T11:57:00.000Z",
          }],
          fingerprint: { state: "unknown", templateSource: "compiled", since: null, diffPresent: false },
        },
        noTelemetryContribution,
        {
          provider: "zai",
          declaredShare: 0.1,
          health: { state: "fresh" },
          capacity: [{
            window: "7d_sonnet", usedFraction: null, usedUnits: null, limitUnits: null,
            resetsAt: null, source: "observed_429", observedAt: "2026-08-07T11:56:00.000Z",
          }],
          fingerprint: { state: "unknown", templateSource: "compiled", since: null, diffPresent: false },
        },
      ],
    },
  },
  me: {
    status: "reported",
    value: {
      displayName: "Ada",
      reports: [
        {
          provider: "anthropic", limitFraction: 0.6, fromDecree: true, consumedUnits: 11,
          drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
        },
        {
          provider: "openai-codex", limitFraction: 0.4, fromDecree: false, consumedUnits: 3,
          drawUnits: 4.5, exceeded: true, enforceable: true, resetsAt: "2026-08-08T00:00:00.000Z",
        },
        {
          provider: "anthropic", limitFraction: 0.2, fromDecree: false, consumedUnits: 0,
          drawUnits: 0, exceeded: false, enforceable: true, resetsAt: "2026-08-09T00:00:00.000Z",
        },
      ],
    },
  },
  fingerprints: {
    status: "reported",
    value: {
      anthropic: {
        templateSource: "compiled", capturedAt: "2026-08-07T10:00:00.000Z", capturePresent: true,
        holdReason: "fingerprint drift", heldAt: "2026-08-07T10:30:00.000Z", diffPresent: true,
      },
      codex: {
        templateSource: "override", capturedAt: null, capturePresent: false,
        holdReason: null, heldAt: null, diffPresent: false,
      },
    },
  },
  models: {
    status: "reported",
    value: {
      models: [
        {
          id: "gpt-5.5", provider: "openai-codex", surface: "codex", upstreamModel: null,
          contextWindow: 200_000, maxTokens: 16_384, reasoning: true, available: true,
        },
        {
          id: "claude-sonnet-4-5", provider: "anthropic", surface: "messages", upstreamModel: null,
          contextWindow: 200_000, maxTokens: 8_192, reasoning: true, available: false,
        },
        {
          id: "k3", provider: "kimi-coding", surface: "messages", upstreamModel: null,
          contextWindow: 1_000_000, maxTokens: 32_768, reasoning: true, available: true,
        },
      ],
    },
  },
};
