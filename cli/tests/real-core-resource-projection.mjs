import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";

import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError, createClient } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorIdSchema,
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySchema,
  AdapterControlService,
  AdapterIdSchema,
  AdapterReconciliationStrength,
  AdapterRegistrationSchema,
  AdapterSnapshotSupport,
  AdapterTargetCategory,
  AttachRequestSchema,
  AttachmentMethodSchema,
  AuthorityDomainIdSchema,
  EndpointIdSchema,
  GenerationSchema,
  GrantIdSchema,
  GrantProvenanceSchema,
  GrantRevocationPolicy,
  GrantSchema,
  LoadSecuritySnapshotRequestSchema,
  IdempotencyStrength,
  LsnSchema,
  ObservationRequestSchema,
  OperationKind,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ResourceCapabilitySchema,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  ResourceProjectionContractSchema,
  ResourceReportMutationSchema,
  ResourceReportSchema,
  ResourceSnapshotReportSchema,
  ResourceSnapshotSchema,
  ResourceStateUpsertSchema,
  ResourceViewReportSchema,
  ReconciliationAction,
  SchemaDescriptorSchema,
  SnapshotViewKind,
  StoredEventKind,
  StoredEventPayloadSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";

import { makeControlClient } from "../dist/src/core-client.js";
import { CredentialStore } from "../dist/src/credentials.js";

const cliRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const repo = resolve(cliRoot, "..");
const binary = join(repo, "target/debug/patchbay-core-server");
const stateDirectory = await mkdtemp(join(tmpdir(), "patchbay-cli-resource-core-"));
const database = join(stateDirectory, "core.sqlite3");
const credentialPath = join(stateDirectory, "credentials.json");
const domainId = "default";
const operatorId = "operator-exact-resource";
const coreSecret = "cli-resource-core-secret";
const adapterSecret = "cli-resource-adapter-secret";
const adapterId = "token-commune";
const poolIdentity = resourceIdentity("token-commune.provider-pool", "pool-opaque");
const drawIdentity = resourceIdentity("token-commune.member-draw", "draw-opaque");
const [corePort, adminPort] = await distinctPorts();
const coreAddress = `http://127.0.0.1:${corePort}`;
const adminAddress = `http://127.0.0.1:${adminPort}`;
const cliEnv = {
  ...process.env,
  PATCHBAY_CORE_SECRET: coreSecret,
  PATCHBAY_CORE_ADDR: coreAddress,
  PATCHBAY_CORE_ADMIN_ADDR: adminAddress,
  PATCHBAY_CREDENTIALS_PATH: credentialPath,
};

let core;
try {
  core = await startCore(true);
  const setup = await runCli([
    "setup",
    "--operator-id", operatorId,
    "--endpoint-id", "cli-bootstrap",
    "--device-id", "cli-resource-host",
  ], {
    PATCHBAY_SETUP_SECRET: core.setupSecret,
    PATCHBAY_OPERATOR_PASSWORD: "exact-resource-password",
  });
  assert.equal(setup.code, 0, setup.stderr);

  await reportResources();
  await stopCore(core.child);
  core = undefined;
  replaceBootstrapGrantWithExactResourceGrant();

  core = await startCore(false);
  const login = await runCli([
    "login",
    "--operator-id", operatorId,
    "--endpoint-id", "cli-exact-resource",
    "--device-id", "cli-resource-host",
  ], { PATCHBAY_OPERATOR_PASSWORD: "exact-resource-password" });
  assert.equal(login.code, 0, login.stderr);

  const credentials = new CredentialStore(credentialPath);
  const client = makeControlClient(coreAddress, coreSecret, credentials);
  await assert.rejects(
    client.loadSecuritySnapshot(create(LoadSecuritySnapshotRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    })),
    (error) => error instanceof ConnectError && error.code === Code.PermissionDenied,
    "the exact resource grant must not authorize the authority-domain security inventory",
  );

  const projection = await runCli(["resource-query", "--json"]);
  assert.equal(projection.code, 0, projection.stderr);
  assert.doesNotMatch(projection.stderr, /permission|authorization_denied/i);
  const json = JSON.parse(projection.stdout);
  assert.equal(json.summaries.length, 1);
  assert.equal(json.summaries[0].provider, "openai-codex");
  assert.equal(json.summaries[0].draw.state, "unavailable");

  const response = await client.loadSnapshot({
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    viewKind: SnapshotViewKind.RESOURCE,
  });
  const decoded = fromBinary(ResourceSnapshotSchema, response.snapshotPayload);
  assert.equal(decoded.resources.length, 1);
  assert.equal(decoded.resources[0].identity?.resourceId?.value, "pool-opaque");

  console.log("CLI real-core resource projection: exact RESOURCE/query grant succeeded; authority-domain security read denied");
} finally {
  if (core) await stopCore(core.child);
  await rm(stateDirectory, { recursive: true, force: true });
}

async function reportResources() {
  let attachmentToken;
  const authenticate = (next) => async (request) => {
    request.header.set("x-patchbay-adapter-id", adapterId);
    request.header.set("x-patchbay-adapter-evidence", adapterSecret);
    if (attachmentToken) request.header.set("x-patchbay-adapter-attachment-token", attachmentToken);
    const response = await next(request);
    attachmentToken = response.header.get("x-patchbay-adapter-attachment-token") ?? attachmentToken;
    return response;
  };
  const client = createClient(AdapterControlService, createGrpcTransport({
    baseUrl: coreAddress,
    interceptors: [authenticate],
  }));
  const domain = create(AuthorityDomainIdSchema, { value: domainId });
  const generation = create(GenerationSchema, { value: 1n });
  const resources = [
    resourceCapability("token-commune.provider-pool", "patchbay.token_commune.provider_pool"),
    resourceCapability("token-commune.member-draw", "patchbay.token_commune.member_draw"),
  ];
  const attached = await client.attach(create(AttachRequestSchema, {
    registration: create(AdapterRegistrationSchema, {
      adapterId: create(AdapterIdSchema, { value: adapterId }),
      endpointId: create(EndpointIdSchema, { value: "token-commune-endpoint" }),
      authorityDomainId: domain,
      adapterGeneration: generation,
      capability: create(AdapterCapabilitySchema, {
        targetCategories: [AdapterTargetCategory.OPERATIONAL_RESOURCE],
        resourceCapabilities: resources,
        assurance: create(AdapterAssuranceManifestSchema, {
          contract: {
            case: "v1",
            value: create(AdapterAssuranceManifestV1Schema, {
              deduplicationStrength: IdempotencyStrength.NONE,
              continuationProofSupport: false,
              cursorSupport: false,
              generationFenceSupport: false,
              reconciliationStrength: AdapterReconciliationStrength.NONE,
              unprovenOutcomeAction: ReconciliationAction.NONE,
            }),
          },
        }),
        attachmentMethod: create(AttachmentMethodSchema, {
          kind: "configured-local-material",
          descriptorContentType: PayloadContentType.BINARY,
        }),
      }),
    }),
    attachmentEvidence: new TextEncoder().encode(adapterSecret),
  }));
  assert.equal(attached.accepted, true);
  assert.ok(attachmentToken);

  await client.ingestObservation(create(ObservationRequestSchema, {
    authorityDomainId: domain,
    observation: {
      case: "resourceReport",
      value: create(ResourceReportSchema, {
        adapterId: create(AdapterIdSchema, { value: adapterId }),
        adapterGeneration: generation,
        observedAt: timestampFromDate(new Date("2026-08-08T10:00:00Z")),
        report: {
          case: "snapshot",
          value: create(ResourceSnapshotReportSchema, {
            views: [
              create(ResourceViewReportSchema, {
                resourceKind: create(ResourceKindSchema, { value: "token-commune.provider-pool" }),
                completeness: AdapterSnapshotSupport.PARTIAL,
                mutations: [upsert(poolIdentity, "patchbay.token_commune.provider_pool", poolProjection())],
              }),
              create(ResourceViewReportSchema, {
                resourceKind: create(ResourceKindSchema, { value: "token-commune.member-draw" }),
                completeness: AdapterSnapshotSupport.PARTIAL,
                mutations: [upsert(drawIdentity, "patchbay.token_commune.member_draw", drawProjection())],
              }),
            ],
          }),
        },
      }),
    },
  }));
}

function replaceBootstrapGrantWithExactResourceGrant() {
  const db = new DatabaseSync(database);
  try {
    const rows = db.prepare("SELECT lsn FROM events WHERE authority_domain_id = ? AND kind = ?").all(
      domainId,
      StoredEventKind.GRANT,
    );
    assert.equal(rows.length, 1, "bootstrap must create exactly one ordinary grant fixture");
    const grant = create(GrantSchema, {
      grantId: create(GrantIdSchema, { value: "exact-pool-query" }),
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      subjectActorId: create(ActorIdSchema, { value: operatorId }),
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RESOURCE,
        resource: poolIdentity,
      }),
      allowedOperationKinds: [OperationKind.QUERY],
      provenance: create(GrantProvenanceSchema, { reason: "real-core CLI regression fixture" }),
      revocationPolicy: GrantRevocationPolicy.CONTINUE,
    });
    const stored = create(StoredEventPayloadSchema, {
      kind: StoredEventKind.GRANT,
      payload: toBinary(GrantSchema, grant),
    });
    db.exec("BEGIN IMMEDIATE");
    try {
      const eventUpdate = db.prepare("UPDATE events SET payload = ? WHERE lsn = ?").run(
        Buffer.from(toBinary(StoredEventPayloadSchema, stored)),
        rows[0].lsn,
      );
      const identityUpdate = db.prepare(
        "UPDATE grant_identities SET grant_id = ? WHERE authority_domain_id = ? AND source_lsn = ?",
      ).run("exact-pool-query", domainId, rows[0].lsn);
      assert.equal(eventUpdate.changes, 1, "grant fixture must replace one authoritative event");
      assert.equal(identityUpdate.changes, 1, "grant fixture must replace its derived identity row");
      db.exec("COMMIT");
    } catch (error) {
      db.exec("ROLLBACK");
      throw error;
    }
  } finally {
    db.close();
  }
}

function resourceCapability(kind, schemaPrefix) {
  return create(ResourceCapabilitySchema, {
    resourceKind: create(ResourceKindSchema, { value: kind }),
    snapshotSupport: AdapterSnapshotSupport.PARTIAL,
    projectionContract: create(ResourceProjectionContractSchema, {
      targetCategory: AdapterTargetCategory.OPERATIONAL_RESOURCE,
      payloadSchema: create(SchemaDescriptorSchema, {
        schemaRef: `${schemaPrefix}.payload.v1`,
        contentType: PayloadContentType.JSON,
      }),
      projectionSchema: create(SchemaDescriptorSchema, {
        schemaRef: `${schemaPrefix}.projection.v1`,
        contentType: PayloadContentType.JSON,
      }),
    }),
  });
}

function upsert(identity, schemaPrefix, projection) {
  return create(ResourceReportMutationSchema, {
    identity,
    mutation: {
      case: "upsert",
      value: create(ResourceStateUpsertSchema, {
        resourcePayload: envelope(`${schemaPrefix}.payload.v1`, {}),
        projectionPayload: envelope(`${schemaPrefix}.projection.v1`, projection),
      }),
    },
  });
}

function envelope(schemaRef, value) {
  return create(PayloadEnvelopeSchema, {
    schemaRef,
    contentType: PayloadContentType.JSON,
    payload: new TextEncoder().encode(JSON.stringify(value)),
  });
}

function resourceIdentity(resourceKind, resourceId) {
  return create(ResourceIdentitySchema, {
    adapterId: create(AdapterIdSchema, { value: adapterId }),
    resourceKind: create(ResourceKindSchema, { value: resourceKind }),
    resourceId: create(ResourceIdSchema, { value: resourceId }),
  });
}

function poolProjection() {
  return {
    provider: "openai-codex",
    contributionListing: {
      status: "reported",
      contributions: [{
        subKey: "local:anonymous-contribution:0123456789abcdef01234567:1",
        subKeySource: "synthesized-content-hash",
        subKeyStability: "snapshot-local",
        attribution: "unavailable",
        declaredShare: 1,
        health: { state: "fresh" },
        telemetryState: "readings",
        capacityReadings: [{
          window: "5h", usedFraction: 0.35, usedUnits: 35, limitUnits: 100,
          resetsAt: null, source: "headers", observedAt: "2026-08-08T10:00:00Z",
        }],
        fingerprint: { state: "ok", templateSource: "compiled", since: null, diffPresent: false },
      }],
    },
    credentialHealthCounts: { fresh: 1, exhausted: 0, authBroken: 0 },
    totalDeclaredShare: 1,
    statusTelemetry: { status: "not-reported", contributions: [] },
    modelCatalog: { status: "reported", models: [{
      id: "gpt-5.5", provider: "openai-codex", surface: "codex", upstreamModel: null,
      contextWindow: 200000, maxTokens: 8192, reasoning: true, available: true,
    }] },
    fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
    capacityAggregation: "none",
  };
}

function drawProjection() {
  return {
    memberDisplayName: "private-member-name",
    provider: "openai-codex",
    reports: [{
      provider: "openai-codex", limitFraction: 0.25, fromDecree: false,
      consumedUnits: 4, drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
    }],
  };
}

async function startCore(expectSetupSecret) {
  const child = spawn(binary, [], {
    cwd: repo,
    env: {
      ...process.env,
      PATCHBAY_CORE_SECRET: coreSecret,
      PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS: JSON.stringify({ [adapterId]: adapterSecret }),
      PATCHBAY_BIND_ADDR: `127.0.0.1:${corePort}`,
      PATCHBAY_ADMIN_BIND_ADDR: `127.0.0.1:${adminPort}`,
      PATCHBAY_DB_PATH: database,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  const deadline = Date.now() + 10_000;
  while (!stdout.includes("patchbay-core-server: local admin h2c")) {
    if (child.exitCode !== null) throw new Error(`core exited before listening (${child.exitCode}): ${stderr}`);
    if (Date.now() >= deadline) throw new Error(`core did not start: ${stderr}`);
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 20));
  }
  const setupSecret = stdout.match(/one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/)?.[1];
  if (expectSetupSecret) assert.ok(setupSecret, `core did not print setup secret: ${stdout}`);
  return { child, setupSecret };
}

async function stopCore(child) {
  if (child.exitCode !== null) return;
  child.kill("SIGTERM");
  await once(child, "exit");
}

function runCli(args, secretEnv = {}) {
  return new Promise((resolveResult, reject) => {
    const child = spawn(process.execPath, [join(cliRoot, "dist/src/main.js"), ...args], {
      cwd: cliRoot,
      env: { ...cliEnv, ...secretEnv },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.once("error", reject);
    child.once("exit", (code) => resolveResult({ code, stdout: stdout.trim(), stderr: stderr.trim() }));
  });
}

async function distinctPorts() {
  const first = await freePort();
  let second = await freePort();
  while (second === first) second = await freePort();
  return [first, second];
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close((error) => error ? reject(error) : resolvePort(port));
    });
  });
}
