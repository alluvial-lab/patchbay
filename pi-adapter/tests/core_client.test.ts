import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http2";
import type { AddressInfo } from "node:net";
import test from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import { connectNodeAdapter } from "@connectrpc/connect-node";
import {
  AdapterControlService,
  AdapterReconciliationStrength,
  AdapterSnapshotSupport,
  AdapterTargetCategory,
  AttachResultSchema,
  AuthorityDomainIdSchema,
  EventIdSchema,
  FailureCode,
  GenerationSchema,
  IdempotencyStrength,
  LsnSchema,
  ObservationResultSchema,
  OperationKind,
  PayloadContentType,
  PiControlProofKind,
  PiCursorDurabilityCondition,
  PiCursorMechanism,
  PiCwdProofKind,
  PiLiveEventCaveat,
  PiPreMaterializationState,
  PiProcessReplacementOnlyKind,
  PiProjectContextResolution,
  PiReloadAdmission,
  PiReloadMechanism,
  PiReloadableResourceKind,
  PiRuntimeProfileSchema,
  PiSessionMaterializationPolicy,
  PiTransportMechanism,
  ReconciliationAction,
  SessionActivityState,
  SessionConnectivityState,
  type PiRuntimeProfile,
  type SessionReport,
} from "@patchbay/contracts";
import {
  decodePiRuntimeProfile,
  PatchbayCoreClient,
  PI_RUNTIME_PROFILE_SCHEMA_REF,
  piCapabilityManifest,
  type SessionIdentity,
} from "../src/core_client.js";
import type { SessionReportOrder } from "../src/session_report_sequencer.js";

const attachmentTokenHeader = "x-patchbay-adapter-attachment-token";

test("Pi manifest declares one complete conservative assurance V1", () => {
  const manifest = piCapabilityManifest();
  assert.equal(manifest.idempotencyStrength, IdempotencyStrength.UNSPECIFIED);
  assert.deepEqual(manifest.targetCategories, [AdapterTargetCategory.RUNTIME_SESSION]);
  assert.equal(manifest.sessionSnapshotSupport, AdapterSnapshotSupport.PARTIAL);
  assert.equal(manifest.sessionReplacementSupport, false);
  assert.equal(manifest.supportedOperationKinds.includes(OperationKind.SPAWN), false);
  assert.deepEqual(manifest.supportedTargetSpecShapes, []);
  assert.equal(manifest.assurance?.contract.case, "v1");
  if (manifest.assurance?.contract.case !== "v1") assert.fail("Pi assurance V1 is required");
  assert.deepEqual(manifest.assurance.contract.value, {
    $typeName: "patchbay.AdapterAssuranceManifestV1",
    deduplicationStrength: IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
    continuationProofSupport: false,
    cursorSupport: false,
    generationFenceSupport: false,
    reconciliationStrength: AdapterReconciliationStrength.NONE,
    unprovenOutcomeAction: ReconciliationAction.MANUAL_REQUIRED,
  });
  assert.deepEqual(manifest.knownFailureModes, [
    FailureCode.UNSUPPORTED_COMMAND,
    FailureCode.EXECUTION_FAILED,
    FailureCode.EXECUTION_OUTCOME_UNKNOWN,
  ]);

  assert.equal(manifest.adapterProfile?.schemaRef, PI_RUNTIME_PROFILE_SCHEMA_REF);
  assert.equal(manifest.adapterProfile?.contentType, PayloadContentType.PROTOBUF);
  const profile = decodePiRuntimeProfile(manifest);
  assert.equal(profile.transport, PiTransportMechanism.RPC_JSONL_SUBPROCESS);
  assert.deepEqual(profile.events?.liveEventCaveats, [
    PiLiveEventCaveat.PARTIAL_ORDER,
    PiLiveEventCaveat.PERSISTED_ENTRIES_REQUIRED_FOR_RECONCILIATION,
  ]);
  assert.equal(
    profile.sessionDurability?.materializationPolicy,
    PiSessionMaterializationPolicy.AFTER_FIRST_ASSISTANT_MESSAGE,
  );
  assert.equal(
    profile.sessionDurability?.preMaterializationState,
    PiPreMaterializationState.MEMORY_ONLY_NOT_RESUMABLE,
  );
  assert.equal(
    profile.cursor?.mechanism,
    PiCursorMechanism.PERSISTED_ENTRY_ID_WITH_EXACT_SET_REPLACEMENT,
  );
  assert.equal(
    profile.cursor?.durabilityCondition,
    PiCursorDurabilityCondition.MATERIALIZED_SESSION_ONLY,
  );
  assert.equal(profile.controlProof?.kind, PiControlProofKind.CHALLENGED_EXTENSION_CUSTOM_ENTRY);
  assert.equal(profile.reload?.mechanism, PiReloadMechanism.CONTROL_EXTENSION_CTX_RELOAD);
  assert.equal(profile.reload?.admission, PiReloadAdmission.IDLE_MATERIALIZED_SESSION);
  assert.deepEqual(profile.reload?.processReplacementOnly, [
    PiProcessReplacementOnlyKind.ARBITRARY_EXTENSION_DEPENDENCY_GRAPH,
    PiProcessReplacementOnlyKind.PI_RUNTIME_PACKAGE_DIST,
    PiProcessReplacementOnlyKind.NATIVE_DEPENDENCY,
    PiProcessReplacementOnlyKind.EXECUTABLE,
    PiProcessReplacementOnlyKind.UNKNOWN_SCOPE,
  ]);
  assert.deepEqual(profile.enumeratedResources, [
    PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
    PiReloadableResourceKind.SKILL,
    PiReloadableResourceKind.PROMPT,
    PiReloadableResourceKind.THEME,
    PiReloadableResourceKind.CONTEXT_FILE,
  ]);
  assert.equal(
    profile.projectContext?.resolution,
    PiProjectContextResolution.ADAPTER_RESOLVED_CWD_PROJECT_TRUST_AND_RESOURCE_ROOTS,
  );
  assert.equal(profile.projectContext?.cwdProof, PiCwdProofKind.CHALLENGED_CONTROL_EXTENSION);
});

test("Pi profile is required and malformed framing fails the adapter-owned decoder", () => {
  const missing = structuredClone(piCapabilityManifest());
  missing.adapterProfile = undefined;
  assert.throws(() => decodePiRuntimeProfile(missing), /missing its runtime profile/);

  const malformed = structuredClone(piCapabilityManifest());
  assert.ok(malformed.adapterProfile);
  malformed.adapterProfile.payload = new Uint8Array([0]);
  assert.throws(() => decodePiRuntimeProfile(malformed), /malformed/);

  const empty = structuredClone(piCapabilityManifest());
  assert.ok(empty.adapterProfile);
  empty.adapterProfile.payload = new Uint8Array();
  assert.throws(() => decodePiRuntimeProfile(empty), /invalid runtime profile envelope/);
});

test("semantically decodable invalid Pi profiles fail the adapter-owned validator", () => {
  const invalidProfiles: ReadonlyArray<{
    name: string;
    mutate: (profile: PiRuntimeProfile) => void;
  }> = [
    {
      name: "unspecified scalar",
      mutate: (profile) => {
        profile.transport = PiTransportMechanism.UNSPECIFIED;
      },
    },
    {
      name: "required nested message absent",
      mutate: (profile) => {
        profile.sessionDurability = undefined;
      },
    },
    {
      name: "repeated exact set contains a duplicate",
      mutate: (profile) => {
        profile.enumeratedResources.push(PiReloadableResourceKind.SKILL);
      },
    },
    {
      name: "repeated exact set omits a required member",
      mutate: (profile) => {
        profile.enumeratedResources = profile.enumeratedResources.filter(
          (kind) => kind !== PiReloadableResourceKind.CONTEXT_FILE,
        );
      },
    },
  ];

  for (const { name, mutate } of invalidProfiles) {
    const manifest = piCapabilityManifest();
    assert.ok(manifest.adapterProfile);
    const profile = fromBinary(PiRuntimeProfileSchema, manifest.adapterProfile.payload);
    mutate(profile);
    manifest.adapterProfile.payload = toBinary(PiRuntimeProfileSchema, profile);

    assert.throws(
      () => decodePiRuntimeProfile(manifest),
      /Pi runtime profile has (?:an unsupported|invalid)/,
      name,
    );
  }
});

test("PatchbayCoreClient reattach retry reuses the exact immutable session report cursor", async () => {
  const attachGenerations: bigint[] = [];
  const attemptedReports: SessionReport[] = [];
  let attachment = 0;
  const handler = connectNodeAdapter({
    routes(router) {
      router.service(AdapterControlService, {
        attach(request, context) {
          attachGenerations.push(request.registration?.adapterGeneration?.value ?? 0n);
          attachment += 1;
          context.responseHeader.set(attachmentTokenHeader, `attachment-${attachment}`);
          return create(AttachResultSchema, {
            accepted: true,
            attachEventId: create(EventIdSchema, {
              authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
              lsn: create(LsnSchema, { value: BigInt(attachment) }),
            }),
          });
        },
        ingestObservation(request) {
          assert.equal(request.observation.case, "sessionReport");
          attemptedReports.push(structuredClone(request.observation.value));
          if (attemptedReports.length === 1) {
            throw new ConnectError("attachment expired", Code.Unauthenticated);
          }
          return create(ObservationResultSchema, {
            eventId: create(EventIdSchema, {
              authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
              lsn: create(LsnSchema, { value: 3n }),
            }),
          });
        },
      });
    },
  });
  const server = createServer(handler);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address() as AddressInfo;

  const client = new PatchbayCoreClient({
    coreAddress: `http://127.0.0.1:${address.port}`,
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
  });
  const identity: SessionIdentity = Object.freeze({
    runtimeSessionId: "runtime-1",
    deploymentScope: "machine-a",
    generation: 4,
    project: "patchbay",
    cwd: "/work/patchbay",
    name: "main",
    model: "provider/model-a",
  });
  const sourceOrder: SessionReportOrder = Object.freeze({
    adapterGeneration: 7,
    revision: 9n,
  });

  try {
    await client.attach(7);
    await client.reportSession(
      identity,
      SessionActivityState.WORKING,
      SessionConnectivityState.LIVE,
      sourceOrder,
    );

    assert.deepEqual(attachGenerations, [7n, 7n]);
    assert.equal(attemptedReports.length, 2);
    assert.deepEqual(
      attemptedReports[1],
      attemptedReports[0],
      "reattach must retry the captured report rather than allocate or reread state",
    );
    assert.deepEqual(attemptedReports[1]?.sourceCursor, {
      $typeName: "patchbay.SessionReportSourceCursor",
      adapterGeneration: create(GenerationSchema, { value: 7n }),
      revision: 9n,
    });
    assert.equal(attemptedReports[1]?.model, "provider/model-a");
  } finally {
    server.close();
    await once(server, "close");
  }
});
