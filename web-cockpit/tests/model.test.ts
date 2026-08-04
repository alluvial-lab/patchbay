import assert from "node:assert/strict";
import test from "node:test";

import { create, toBinary, type DescMessage, type MessageShape } from "@bufbuild/protobuf";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  AdapterSnapshotSupport,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  CommandTransitionSchema,
  ElicitationIdSchema,
  ElicitationResponsePayloadSchema,
  ElicitationSchema,
  ElicitationState,
  EventIdSchema,
  GenerationSchema,
  LsnSchema,
  ObservationKind,
  ObservationSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  QuestionContractSchema,
  ResponseContractKind,
  ResponseContractSchema,
  ResponseOptionSchema,
  ResourceFreshnessChangedSchema,
  ResourceFreshnessState,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  ResourceSchema,
  ResourceSnapshotSchema,
  ResourceStateEventSchema,
  ResourceStateMutationSchema,
  ResourceStateTombstoneSchema,
  ResourceStateUnknownSchema,
  ResourceStateUpsertSchema,
  ResourceViewRevisionSchema,
  ResourceViewStateUpdateSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionGenerationBumpedSchema,
  SessionModelChangedSchema,
  SessionRegisteredSchema,
  SessionSchema,
  SessionSnapshotSchema,
  SessionStateEventSchema,
  SecurityLockdownEventSchema,
  SessionStateSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubscribeEventSchema,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
  type ResourceIdentity,
  type ResourceStateMutation,
  type SubscribeEvent,
} from "@patchbay/contracts";
import fc from "fast-check";

import {
  PresentationProjection,
  emptyPresentationModel,
  fold,
  markUnreconciled,
  operationTargetFromScope,
  rendersLive,
  rendersResourceCurrent,
  replaceFromSnapshots,
  resourceCollectionKey,
  resourceKey,
  sessionKey,
} from "../src/domain/model.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });
const adapterId = create(AdapterIdSchema, { value: "pi" });
const runtimeSessionId = create(RuntimeSessionIdSchema, { value: "session-1" });
const deploymentScope = "laptop";
const encoder = new TextEncoder();

test("fold is pure and registration exposes the stable identity tuple", () => {
  const initial = emptyPresentationModel();
  const next = fold(initial, registration(1n, 1n));

  assert.equal(initial.sessions.size, 0);
  assert.equal(initial.cursor, 0n);
  assert.notEqual(next, initial);
  assert.equal(next.sessions.size, 1);
  const session = [...next.sessions.values()][0]!;
  assert.deepEqual(session.identity, {
    adapterId: "pi",
    deploymentScope,
    runtimeSessionId: "session-1",
    generation: 1n,
  });
  assert.equal(rendersLive(session), true);
  assert.equal(session.model, "provider/model-1");
});

test("lockdown folds to an inline read-only posture and stale-dominant sessions", () => {
  let model = fold(emptyPresentationModel(), registration(1n, 1n));
  model = fold(model, stored(
    2n,
    StoredEventKind.SECURITY_LOCKDOWN,
    SecurityLockdownEventSchema,
    create(SecurityLockdownEventSchema, {
      authorityDomainId: DOMAIN,
      transition: {
        case: "entered",
        value: {
          reasonCode: "suspected_endpoint_compromise",
          occurredAt: { seconds: 1n, nanos: 0 },
          invalidatedThroughOperatorSessionGeneration: { value: 3n },
          affectedRuntimeSessionCount: 1,
        },
      },
    }),
  ));
  assert.equal(model.lockdown.active, true);
  assert.equal(model.lockdown.reasonCode, "suspected_endpoint_compromise");
  const locked = [...model.sessions.values()][0]!;
  assert.equal(locked.connectivity, SessionConnectivityState.STALE);
  assert.equal(locked.lockdownActive, true);
  assert.equal(rendersLive(locked), false);

  model = fold(model, stored(
    3n,
    StoredEventKind.SECURITY_LOCKDOWN,
    SecurityLockdownEventSchema,
    create(SecurityLockdownEventSchema, {
      authorityDomainId: DOMAIN,
      transition: { case: "exited", value: { reasonCode: "bootstrap_exit", bootstrapChannel: 1 } },
    }),
  ));
  assert.equal(model.lockdown.active, false);
  assert.equal(rendersLive([...model.sessions.values()][0]!), false);
});

test("model deltas preserve session identity", () => {
  const registered = fold(emptyPresentationModel(), registration(1n, 1n));
  const updated = fold(registered, modelChange(2n, "provider/model-1", "provider/model-2"));
  const session = [...updated.sessions.values()][0]!;
  assert.equal(session.model, "provider/model-2");
  assert.deepEqual(session.identity, {
    adapterId: "pi",
    deploymentScope,
    runtimeSessionId: "session-1",
    generation: 1n,
  });
});

test("unreconciled and tombstoned generations cannot render as live", () => {
  const live = fold(emptyPresentationModel(), registration(1n, 1n));
  const stale = markUnreconciled(live);
  assert.equal(rendersLive([...stale.sessions.values()][0]!), false);
  assert.equal(rendersLive([...live.sessions.values()][0]!), true);

  const bumped = fold(live, generationBump(2n, 1n, 2n));
  const old = bumped.sessions.get(
    sessionKey({ adapterId: "pi", deploymentScope, runtimeSessionId: "session-1", generation: 1n }),
  )!;
  assert.equal(old.tombstoned, true);
  assert.equal(rendersLive(old), false);
  assert.equal(rendersLive([...bumped.sessions.values()].find((session) => !session.tombstoned)!), true);
});

test("generation monotonicity holds across generated strictly increasing bumps", async () => {
  await fc.assert(
    fc.asyncProperty(
      fc.array(fc.integer({ min: 1, max: 10 }), { minLength: 1, maxLength: 30 }),
      async (increments) => {
        let model = fold(emptyPresentationModel(), registration(1n, 1n));
        let generation = 1n;
        let lsn = 2n;
        for (const increment of increments) {
          const nextGeneration = generation + BigInt(increment);
          model = fold(model, generationBump(lsn, generation, nextGeneration));
          const live = [...model.sessions.values()].filter((session) => !session.tombstoned);
          assert.equal(live.length, 1);
          assert.equal(live[0]!.identity.generation, nextGeneration);
          for (const stale of [...model.sessions.values()].filter((session) => session.tombstoned)) {
            assert.equal(rendersLive(stale), false);
          }
          generation = nextGeneration;
          lsn += 1n;
        }
      },
    ),
    { numRuns: 100 },
  );
});

test("Observation detail composes without changing durable activity", () => {
  const registered = fold(emptyPresentationModel(), registration(1n, 1n));
  const next = fold(
    registered,
    observationEvent(2n, {
      kind: "tool_requested",
      eventId: "event-2",
      sessionId: "session-1",
      ts: 2,
      toolCallId: "tool-1",
      tool: "bash",
      args: {},
    }),
  );
  const session = [...next.sessions.values()][0]!;
  assert.equal(session.activity, SessionActivityState.WORKING);
  assert.equal(session.activityDetail, "using bash");
  assert.equal(registered.sessions.values().next().value!.activityDetail, undefined);
});

test("Operation targets project exact runtime-session or operational-resource identities", () => {
  const session = operationTargetFromScope(sessionTarget(1n));
  assert.deepEqual(session, {
    kind: "runtime-session",
    identity: {
      adapterId: "pi",
      deploymentScope,
      runtimeSessionId: "session-1",
      generation: 1n,
    },
  });

  const identity = resourceIdentity("pool-target");
  const scope = create(TargetScopeSchema, {
    kind: TargetScopeKind.RESOURCE,
    resource: identity,
  });
  assert.deepEqual(operationTargetFromScope(scope), {
    kind: "operational-resource",
    identity: { adapterId: "pi", resourceKind: "provider_pool", resourceId: "pool-target" },
  });
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: "resource-command" }),
    authorityDomainId: DOMAIN,
    kind: OperationKind.QUERY,
    targetScope: scope,
    idempotencyKey: "resource-command-key",
  });
  const model = fold(emptyPresentationModel(), stored(
    1n,
    StoredEventKind.OPERATION,
    OperationSchema,
    operation,
  ));
  assert.deepEqual(model.commands.get("resource-command")!.target, operationTargetFromScope(scope));
});

test("partial, mixed, and legacy resource scopes never become command targets", () => {
  const identity = resourceIdentity("pool-target");
  const rejected = [
    create(TargetScopeSchema, {
      kind: TargetScopeKind.RESOURCE,
      resource: create(ResourceIdentitySchema, { adapterId, resourceId: identity.resourceId }),
    }),
    create(TargetScopeSchema, {
      kind: TargetScopeKind.RESOURCE,
      resource: identity,
      adapterId,
    }),
    create(TargetScopeSchema, {
      kind: TargetScopeKind.RESOURCE,
      legacyAuditResourceId: "pool-target",
    }),
    create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId,
      deploymentScope,
      runtimeSessionId,
      sessionGeneration: create(GenerationSchema, { value: 1n }),
      resource: identity,
    }),
  ];
  for (const scope of rejected) assert.equal(operationTargetFromScope(scope), undefined);
});

test("first durable command terminal remains projected after a late terminal candidate", () => {
  let model = fold(emptyPresentationModel(), operationEvent(1n, "command-1"));
  model = fold(model, transitionEvent(2n, "command-1", OperationState.ACCEPTED, OperationState.DELIVERED));
  model = fold(model, transitionEvent(3n, "command-1", OperationState.DELIVERED, OperationState.COMPLETED));
  model = fold(model, transitionEvent(4n, "command-1", OperationState.COMPLETED, OperationState.CANCELLED));

  assert.equal(model.commands.get("command-1")!.state, OperationState.COMPLETED);
  assert.equal(model.commands.get("command-1")!.history.length, 3);
});

test("snapshot replacement discards the old projection and pending elicitations derive needs-you", () => {
  const projection = new PresentationProjection(fold(emptyPresentationModel(), registration(1n, 1n)));
  projection.foldEvent(elicitationEvent(2n));
  assert.equal([...projection.model.sessions.values()][0]!.needsYou, true);

  projection.markUnreconciled();
  assert.equal(rendersLive([...projection.model.sessions.values()][0]!), false);

  projection.replaceFromSnapshots(
    {
      session: create(SessionSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 0n }),
        sessions: [],
      }),
      resource: create(ResourceSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 0n }),
      }),
    },
    [],
  );
  assert.equal(projection.model.sessions.size, 0);
  assert.equal(projection.model.elicitations.size, 0);
  assert.equal(projection.model.commands.size, 0);
  assert.equal(projection.model.cursor, 0n);
});

test("a late Elicitation event cannot rewrite the first terminal state", () => {
  let model = fold(emptyPresentationModel(), questionElicitationEvent(1n, "elicitation-1", ElicitationState.PENDING));
  model = fold(model, questionElicitationEvent(2n, "elicitation-1", ElicitationState.ANSWERED));
  model = fold(model, questionElicitationEvent(3n, "elicitation-1", ElicitationState.PENDING));

  assert.equal(model.elicitations.get("elicitation-1")!.state, ElicitationState.ANSWERED);
  assert.equal(model.elicitations.get("elicitation-1")!.lsn, 2n);
});

test("a second completed response cannot overwrite the first answer", () => {
  let model = fold(emptyPresentationModel(), questionElicitationEvent(1n, "elicitation-1", ElicitationState.PENDING));
  model = fold(model, responseOperationEvent(2n, "response-1", "main"));
  model = fold(model, transitionEvent(3n, "response-1", OperationState.ACCEPTED, OperationState.DELIVERED));
  model = fold(model, transitionEvent(4n, "response-1", OperationState.DELIVERED, OperationState.COMPLETED));
  model = fold(model, responseOperationEvent(5n, "response-2", "feature"));
  model = fold(model, transitionEvent(6n, "response-2", OperationState.ACCEPTED, OperationState.DELIVERED));
  model = fold(model, transitionEvent(7n, "response-2", OperationState.DELIVERED, OperationState.COMPLETED));

  const elicitation = model.elicitations.get("elicitation-1")!;
  assert.equal(elicitation.state, ElicitationState.ANSWERED);
  assert.equal(elicitation.answer?.selectedOptionId, "main");
  assert.equal(elicitation.lsn, 4n);
});

test("question Elicitations preserve same-opener batch correlation as a grouping key", () => {
  let model = fold(emptyPresentationModel(), questionElicitationEvent(1n, "question-1", ElicitationState.PENDING, "batch-command"));
  model = fold(model, questionElicitationEvent(2n, "question-2", ElicitationState.PENDING, "batch-command"));

  const first = model.elicitations.get("question-1")!;
  const second = model.elicitations.get("question-2")!;
  assert.ok(first.groupingKey);
  assert.equal(first.groupingKey, second.groupingKey);
});

function registration(lsn: bigint, generation: bigint): SubscribeEvent {
  const mutation = create(SessionRegisteredSchema, {
    adapterId,
    deploymentScope,
    runtimeSessionId,
    sessionGeneration: create(GenerationSchema, { value: generation }),
    initialState: create(SessionStateSchema, {
      connectivity: SessionConnectivityState.LIVE,
      activity: SessionActivityState.WORKING,
    }),
    project: "patchbay",
    cwd: "/projects/patchbay",
    name: "core",
    model: "provider/model-1",
  });
  return stored(
    lsn,
    StoredEventKind.SESSION_STATE,
    SessionStateEventSchema,
    create(SessionStateEventSchema, {
      authorityDomainId: DOMAIN,
      mutation: { case: "registered", value: mutation },
    }),
  );
}

function generationBump(lsn: bigint, from: bigint, to: bigint): SubscribeEvent {
  const mutation = create(SessionGenerationBumpedSchema, {
    adapterId,
    deploymentScope,
    runtimeSessionId,
    fromGeneration: create(GenerationSchema, { value: from }),
    toGeneration: create(GenerationSchema, { value: to }),
    initialState: create(SessionStateSchema, {
      connectivity: SessionConnectivityState.LIVE,
      activity: SessionActivityState.IDLE,
    }),
    project: "patchbay",
    cwd: "/projects/patchbay",
    name: "core",
    model: "provider/model-2",
  });
  return stored(
    lsn,
    StoredEventKind.SESSION_STATE,
    SessionStateEventSchema,
    create(SessionStateEventSchema, {
      authorityDomainId: DOMAIN,
      mutation: { case: "generationBumped", value: mutation },
    }),
  );
}

function modelChange(lsn: bigint, from: string, to: string): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.SESSION_STATE,
    SessionStateEventSchema,
    create(SessionStateEventSchema, {
      authorityDomainId: DOMAIN,
      mutation: {
        case: "modelChanged",
        value: create(SessionModelChangedSchema, {
          adapterId,
          deploymentScope,
          runtimeSessionId,
          sessionGeneration: create(GenerationSchema, { value: 1n }),
          from,
          to,
        }),
      },
    }),
  );
}

function observationEvent(lsn: bigint, transcript: Record<string, unknown>): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.OBSERVATION,
    ObservationSchema,
    create(ObservationSchema, {
      authorityDomainId: DOMAIN,
      kind: ObservationKind.EVENT,
      targetScope: sessionTarget(1n),
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.JSON,
        schemaRef: "patchbay.pi.TranscriptEvent.v1",
        payload: encoder.encode(JSON.stringify(transcript)),
      }),
    }),
  );
}

function operationEvent(lsn: bigint, commandId: string): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.OPERATION,
    OperationSchema,
    create(OperationSchema, {
      commandId: create(CommandIdSchema, { value: commandId }),
      authorityDomainId: DOMAIN,
      kind: OperationKind.INSTRUCT,
      targetScope: sessionTarget(1n),
      idempotencyKey: `key-${commandId}`,
    }),
  );
}

function transitionEvent(
  lsn: bigint,
  commandId: string,
  fromState: OperationState,
  toState: OperationState,
): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.COMMAND_TRANSITION,
    CommandTransitionSchema,
    create(CommandTransitionSchema, {
      commandId: create(CommandIdSchema, { value: commandId }),
      fromState,
      toState,
    }),
  );
}

function elicitationEvent(lsn: bigint): SubscribeEvent {
  return questionElicitationEvent(lsn, "elicitation-1", ElicitationState.PENDING);
}

function questionElicitationEvent(
  lsn: bigint,
  id: string,
  state: ElicitationState,
  batchCommandId?: string,
): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.ELICITATION,
    ElicitationSchema,
    create(ElicitationSchema, {
      elicitationId: create(ElicitationIdSchema, { value: id }),
      authorityDomainId: DOMAIN,
      opener: create(ActorEndpointRefSchema, {
        actorId: create(ActorIdSchema, { value: "pi-agent" }),
      }),
      targetContext: sessionTarget(1n),
      correlations: batchCommandId
        ? [
            create(TypedCorrelationSchema, {
              ref: {
                case: "commandId",
                value: create(CommandIdSchema, { value: batchCommandId }),
              },
            }),
          ]
        : [],
      responseContract: create(ResponseContractSchema, {
        contractKind: ResponseContractKind.QUESTION,
        contractBody: {
          case: "question",
          value: create(QuestionContractSchema, {
            options: [
              create(ResponseOptionSchema, { optionId: "main", label: "main" }),
              create(ResponseOptionSchema, { optionId: "feature", label: "feature" }),
            ],
          }),
        },
      }),
      state,
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.TEXT_UTF8,
        payload: encoder.encode("Which path?"),
      }),
    }),
  );
}

function responseOperationEvent(lsn: bigint, commandId: string, selectedOptionId: string): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.OPERATION,
    OperationSchema,
    create(OperationSchema, {
      commandId: create(CommandIdSchema, { value: commandId }),
      authorityDomainId: DOMAIN,
      kind: OperationKind.ELICITATION_RESPONSE,
      targetScope: sessionTarget(1n),
      idempotencyKey: `${commandId}-key`,
      correlations: [
        create(TypedCorrelationSchema, {
          ref: {
            case: "elicitationId",
            value: create(ElicitationIdSchema, { value: "elicitation-1" }),
          },
        }),
      ],
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: "patchbay.ElicitationResponsePayload",
        payload: toBinary(
          ElicitationResponsePayloadSchema,
          create(ElicitationResponsePayloadSchema, { selectedOptionId }),
        ),
      }),
    }),
  );
}

test("resource identities and collection keys use collision-proof tuple composition", () => {
  assert.notEqual(
    resourceKey({ adapterId: "a/b", resourceKind: "c", resourceId: "d" }),
    resourceKey({ adapterId: "a", resourceKind: "b/c", resourceId: "d" }),
  );
  assert.notEqual(
    resourceKey({ adapterId: "a", resourceKind: "b", resourceId: "c/d" }),
    resourceKey({ adapterId: "a", resourceKind: "b/c", resourceId: "d" }),
  );
  assert.notEqual(resourceCollectionKey("a/b", "c"), resourceCollectionKey("a", "b/c"));
});

test("resource current styling requires reconciled current non-tombstoned state", () => {
  const base = {
    identity: { adapterId: "token-commune", resourceKind: "provider_pool", resourceId: "pool-1" },
    freshness: ResourceFreshnessState.CURRENT,
    sourceAdapterGeneration: 1n,
    revisionLsn: 4n,
    tombstoned: false,
    hasCachedPayload: true,
    reconciled: true,
    projection: { status: "unavailable" as const },
  };
  assert.equal(rendersResourceCurrent(base), true);
  assert.equal(rendersResourceCurrent({ ...base, reconciled: false }), false);
  assert.equal(rendersResourceCurrent({ ...base, tombstoned: true }), false);
  assert.equal(rendersResourceCurrent({ ...base, freshness: ResourceFreshnessState.STALE }), false);
  assert.equal(rendersResourceCurrent({ ...base, freshness: ResourceFreshnessState.UNKNOWN }), false);
});

test("resource upsert, freshness, unknown, and replacement fold exact identities", () => {
  const pool1 = resourceIdentity("pool-1");
  const pool2 = resourceIdentity("pool-2");
  let model = fold(emptyPresentationModel(), resourceEvent(1n, [
    resourceMutation(pool1, undefined, "upsert"),
  ]));
  let first = model.resources.get(resourceKey({
    adapterId: "pi", resourceKind: "provider_pool", resourceId: "pool-1",
  }))!;
  assert.equal(first.projection.status, "decoded");
  assert.equal(first.freshness, ResourceFreshnessState.CURRENT);

  model = fold(model, resourceEvent(2n, [
    resourceMutation(pool1, 1n, "freshness", ResourceFreshnessState.STALE),
  ]));
  first = model.resources.get(resourceKey(first.identity))!;
  assert.equal(first.freshness, ResourceFreshnessState.STALE);
  assert.equal(first.hasCachedPayload, true);

  model = fold(model, resourceEvent(3n, [resourceMutation(pool1, 2n, "unknown")]));
  first = model.resources.get(resourceKey(first.identity))!;
  assert.equal(first.freshness, ResourceFreshnessState.UNKNOWN);
  assert.equal(first.hasCachedPayload, false);
  assert.equal(first.projection.status, "unavailable");

  model = fold(model, resourceEvent(4n, [resourceMutation(pool1, 3n, "upsert")]));
  model = fold(model, resourceEvent(5n, [
    resourceMutation(pool1, 4n, "tombstone", undefined, pool2),
    resourceMutation(pool2, undefined, "upsert"),
  ]));
  first = model.resources.get(resourceKey(first.identity))!;
  const replacement = model.resources.get(resourceKey({
    adapterId: "pi", resourceKind: "provider_pool", resourceId: "pool-2",
  }))!;
  assert.equal(first.tombstoned, true);
  assert.equal(first.freshness, ResourceFreshnessState.STALE);
  assert.deepEqual(first.replacedBy, replacement.identity);
  assert.equal(replacement.tombstoned, false);
  assert.equal(replacement.freshness, ResourceFreshnessState.CURRENT);
  assert.equal(model.resourceCollections.get(resourceCollectionKey("pi", "provider_pool"))?.revisionLsn, 5n);
  assert.equal(model.sessions.size, 0);
});

test("unequal snapshot horizons replay each state axis only after its own baseline", () => {
  const identity = resourceIdentity("snapshot-pool");
  const upsert = resourceMutation(identity, undefined, "upsert");
  if (upsert.mutation.case !== "upsert") throw new Error("fixture bug");
  const model = replaceFromSnapshots(
    {
      session: create(SessionSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 1n }),
        sessions: [create(SessionSchema, {
          authorityDomainId: DOMAIN,
          adapterId,
          deploymentScope,
          runtimeSessionId,
          sessionGeneration: create(GenerationSchema, { value: 1n }),
          state: create(SessionStateSchema, {
            connectivity: SessionConnectivityState.LIVE,
            activity: SessionActivityState.IDLE,
          }),
          model: "provider/model-1",
        })],
      }),
      resource: create(ResourceSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 3n }),
        resources: [create(ResourceSchema, {
          authorityDomainId: DOMAIN,
          identity,
          resourcePayload: upsert.mutation.value.resourcePayload,
          projectionPayload: upsert.mutation.value.projectionPayload,
          freshness: ResourceFreshnessState.CURRENT,
          sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
          revisionLsn: create(LsnSchema, { value: 3n }),
          observedAt: { seconds: 3n, nanos: 0 },
        })],
        viewRevisions: [create(ResourceViewRevisionSchema, {
          adapterId,
          resourceKind: identity.resourceKind,
          completeness: AdapterSnapshotSupport.AUTHORITATIVE,
          sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
          revisionLsn: create(LsnSchema, { value: 3n }),
          observedAt: { seconds: 3n, nanos: 0 },
        })],
      }),
    },
    [modelChange(2n, "provider/model-1", "provider/model-2")],
  );
  assert.equal(model.cursor, 3n);
  assert.equal([...model.sessions.values()][0]!.model, "provider/model-2");
  assert.equal([...model.resources.values()][0]!.revisionLsn, 3n);
  assert.equal(model.reconciled, true);
  assert.equal([...model.resources.values()][0]!.reconciled, true);
});

test("reverse unequal snapshot horizons replay the resource axis after its own baseline", () => {
  const identity = resourceIdentity("reverse-snapshot-pool");
  const upsert = resourceMutation(identity, undefined, "upsert");
  if (upsert.mutation.case !== "upsert") throw new Error("fixture bug");
  const visiblePrefix = [
    resourceEvent(1n, [upsert]),
    resourceEvent(2n, [resourceMutation(identity, 1n, "freshness", ResourceFreshnessState.STALE)]),
    registration(3n, 1n),
  ];
  const oracle = visiblePrefix.reduce(
    (model, event) => fold(model, event),
    emptyPresentationModel(),
  );
  const rebuilt = replaceFromSnapshots(
    {
      session: create(SessionSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 3n }),
        sessions: [create(SessionSchema, {
          authorityDomainId: DOMAIN,
          adapterId,
          deploymentScope,
          runtimeSessionId,
          sessionGeneration: create(GenerationSchema, { value: 1n }),
          state: create(SessionStateSchema, {
            connectivity: SessionConnectivityState.LIVE,
            activity: SessionActivityState.WORKING,
          }),
          project: "patchbay",
          cwd: "/projects/patchbay",
          name: "core",
          model: "provider/model-1",
        })],
      }),
      resource: create(ResourceSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: create(LsnSchema, { value: 1n }),
        resources: [create(ResourceSchema, {
          authorityDomainId: DOMAIN,
          identity,
          resourcePayload: upsert.mutation.value.resourcePayload,
          projectionPayload: upsert.mutation.value.projectionPayload,
          freshness: ResourceFreshnessState.CURRENT,
          sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
          revisionLsn: create(LsnSchema, { value: 1n }),
          observedAt: { seconds: 1n, nanos: 0 },
        })],
        viewRevisions: [create(ResourceViewRevisionSchema, {
          adapterId,
          resourceKind: identity.resourceKind,
          completeness: AdapterSnapshotSupport.AUTHORITATIVE,
          sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
          revisionLsn: create(LsnSchema, { value: 1n }),
          observedAt: { seconds: 1n, nanos: 0 },
        })],
      }),
    },
    visiblePrefix,
  );

  const resource = rebuilt.resources.get(resourceKey({
    adapterId: "pi",
    resourceKind: "provider_pool",
    resourceId: "reverse-snapshot-pool",
  }))!;
  const oracleResource = oracle.resources.get(resourceKey(resource.identity))!;
  const resourceState = (value: typeof resource) => ({
    identity: value.identity,
    freshness: value.freshness,
    sourceAdapterGeneration: value.sourceAdapterGeneration,
    revisionLsn: value.revisionLsn,
    observedAt: value.observedAt,
    tombstoned: value.tombstoned,
    replacedBy: value.replacedBy,
    hasCachedPayload: value.hasCachedPayload,
    reconciled: value.reconciled,
    projection: value.projection,
  });
  assert.equal(rebuilt.cursor, 3n);
  assert.equal(rebuilt.cursor, oracle.cursor);
  assert.equal(resource.revisionLsn, 2n);
  assert.equal(resource.freshness, ResourceFreshnessState.STALE);
  assert.deepEqual(resourceState(resource), resourceState(oracleResource));
  assert.deepEqual(rebuilt.resourceCollections, oracle.resourceCollections);
  const session = [...rebuilt.sessions.values()][0]!;
  const oracleSession = [...oracle.sessions.values()][0]!;
  assert.deepEqual({
    identity: session.identity,
    label: session.label,
    model: session.model,
    connectivity: session.connectivity,
    activity: session.activity,
    lastLsn: session.lastLsn,
    tombstoned: session.tombstoned,
    reconciled: session.reconciled,
  }, {
    identity: oracleSession.identity,
    label: oracleSession.label,
    model: oracleSession.model,
    connectivity: oracleSession.connectivity,
    activity: oracleSession.activity,
    lastLsn: oracleSession.lastLsn,
    tombstoned: oracleSession.tombstoned,
    reconciled: oracleSession.reconciled,
  });
});

test("invalid replay never installs a partially rebuilt projection", () => {
  const projection = new PresentationProjection(fold(emptyPresentationModel(), operationEvent(1n, "existing")));
  const before = projection.model;
  const malformed = stored(
    1n,
    StoredEventKind.RESOURCE_STATE,
    ResourceStateEventSchema,
    create(ResourceStateEventSchema, {
      authorityDomainId: DOMAIN,
      sourceAdapterId: adapterId,
      sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
    }),
  );
  assert.throws(() => projection.replaceFromSnapshots({
    session: create(SessionSnapshotSchema, {
      authorityDomainId: DOMAIN,
      snapshotLsn: create(LsnSchema, { value: 1n }),
    }),
    resource: create(ResourceSnapshotSchema, {
      authorityDomainId: DOMAIN,
      snapshotLsn: create(LsnSchema, { value: 0n }),
    }),
  }, [malformed]));
  assert.equal(projection.model, before);
  assert.equal(projection.model.commands.has("existing"), true);
});

test("projection decode failure is local while malformed normalized resource events fail closed", () => {
  const identity = resourceIdentity("bad-projection");
  const invalidProjection = resourceMutation(identity, undefined, "upsert");
  if (invalidProjection.mutation.case !== "upsert") throw new Error("fixture bug");
  invalidProjection.mutation.value.projectionPayload!.payload = encoder.encode("{");
  const model = fold(emptyPresentationModel(), resourceEvent(1n, [invalidProjection]));
  assert.equal([...model.resources.values()][0]!.projection.status, "invalid");
  assert.equal(model.cursor, 1n);

  assert.throws(
    () => fold(model, resourceEvent(2n, [resourceMutation(identity, 999n, "unknown")])),
    /prior revision/,
  );
  assert.equal(model.cursor, 1n);
  assert.equal([...model.resources.values()][0]!.revisionLsn, 1n);
});

test("stream gaps preserve cached resource values as stale and empty values as unknown", () => {
  const cached = resourceIdentity("cached");
  const empty = resourceIdentity("empty");
  let model = fold(emptyPresentationModel(), resourceEvent(1n, [
    resourceMutation(cached, undefined, "upsert"),
    resourceMutation(empty, undefined, "unknown"),
  ]));
  model = markUnreconciled(model);
  assert.equal(model.reconciled, false);
  assert.equal(model.resourceCollections.values().next().value!.reconciled, false);
  assert.equal(model.resources.get(resourceKey({ adapterId: "pi", resourceKind: "provider_pool", resourceId: "cached" }))!.freshness, ResourceFreshnessState.STALE);
  assert.equal(model.resources.get(resourceKey({ adapterId: "pi", resourceKind: "provider_pool", resourceId: "empty" }))!.freshness, ResourceFreshnessState.UNKNOWN);
  assert.equal([...model.resources.values()].every((resource) => !resource.reconciled), true);
});

function resourceIdentity(resourceId: string): ResourceIdentity {
  return create(ResourceIdentitySchema, {
    adapterId,
    resourceKind: create(ResourceKindSchema, { value: "provider_pool" }),
    resourceId: create(ResourceIdSchema, { value: resourceId }),
  });
}

function resourceMutation(
  identity: ResourceIdentity,
  fromRevision: bigint | undefined,
  kind: "upsert" | "unknown" | "tombstone" | "freshness",
  freshnessTo?: ResourceFreshnessState,
  replacement?: ResourceIdentity,
): ResourceStateMutation {
  const mutation = kind === "upsert"
    ? {
        case: "upsert" as const,
        value: create(ResourceStateUpsertSchema, {
          resourcePayload: create(PayloadEnvelopeSchema, {
            contentType: PayloadContentType.JSON,
            schemaRef: "provider_pool.payload.v1",
            payload: encoder.encode("{}"),
          }),
          projectionPayload: create(PayloadEnvelopeSchema, {
            contentType: PayloadContentType.JSON,
            schemaRef: "provider_pool.projection.v1",
            payload: encoder.encode(JSON.stringify({
              displayName: identity.resourceId!.value,
              providerLabel: "Provider",
              health: "serving",
              remainingPercent: 75,
            })),
          }),
        }),
      }
    : kind === "unknown"
      ? { case: "unknown" as const, value: create(ResourceStateUnknownSchema) }
      : kind === "tombstone"
        ? { case: "tombstone" as const, value: create(ResourceStateTombstoneSchema, { replacedBy: replacement }) }
        : {
            case: "freshnessChanged" as const,
            value: create(ResourceFreshnessChangedSchema, {
              from: ResourceFreshnessState.CURRENT,
              to: freshnessTo,
            }),
          };
  return create(ResourceStateMutationSchema, {
    identity,
    fromRevisionLsn: fromRevision === undefined ? undefined : create(LsnSchema, { value: fromRevision }),
    mutation,
  });
}

function resourceEvent(lsn: bigint, mutations: ResourceStateMutation[]): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.RESOURCE_STATE,
    ResourceStateEventSchema,
    create(ResourceStateEventSchema, {
      authorityDomainId: DOMAIN,
      sourceAdapterId: adapterId,
      sourceAdapterGeneration: create(GenerationSchema, { value: 1n }),
      views: [create(ResourceViewStateUpdateSchema, {
        resourceKind: create(ResourceKindSchema, { value: "provider_pool" }),
        completeness: AdapterSnapshotSupport.AUTHORITATIVE,
      })],
      mutations,
      observedAt: { seconds: lsn, nanos: 0 },
    }),
  );
}

function sessionTarget(generation: bigint) {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId,
    deploymentScope,
    runtimeSessionId,
    sessionGeneration: create(GenerationSchema, { value: generation }),
  });
}

function stored<D extends DescMessage>(
  lsn: bigint,
  kind: StoredEventKind,
  schema: D,
  message: MessageShape<D>,
): SubscribeEvent {
  return create(SubscribeEventSchema, {
    eventId: create(EventIdSchema, {
      authorityDomainId: DOMAIN,
      lsn: create(LsnSchema, { value: lsn }),
    }),
    payload: create(StoredEventPayloadSchema, {
      kind,
      payload: toBinary(schema, message),
    }),
  });
}

test("tool rows carry an args/result detail preview", () => {
  let model = fold(emptyPresentationModel(), registration(1n, 1n));

  model = fold(model, observationEvent(2n, {
    kind: "tool_requested", eventId: "e1", sessionId: "session-1", ts: 2,
    toolCallId: "t1", tool: "bash", args: { command: "pwd && ls" },
  }));
  model = fold(model, observationEvent(3n, {
    kind: "tool_requested", eventId: "e2", sessionId: "session-1", ts: 3,
    toolCallId: "t2", tool: "read", args: { path: "docs/VISION.md" },
  }));
  model = fold(model, observationEvent(4n, {
    kind: "tool_requested", eventId: "e3", sessionId: "session-1", ts: 4,
    toolCallId: "t3", tool: "mystery", args: { zeta: 1, alpha: "two" },
  }));
  model = fold(model, observationEvent(5n, {
    kind: "tool_finished", eventId: "e4", sessionId: "session-1", ts: 5,
    toolCallId: "t1", tool: "bash", result: "total 42",
  }));
  model = fold(model, observationEvent(6n, {
    kind: "tool_requested", eventId: "e5", sessionId: "session-1", ts: 6,
    toolCallId: "t4", tool: "bash", args: { command: "x".repeat(500) },
  }));

  const rows = model.observations.filter((o) => o.role === "tool");
  const byId = new Map(rows.map((o) => [o.id, o]));

  assert.equal(byId.get("t1")!.detail, "pwd && ls");
  assert.equal(byId.get("t2")!.detail, "docs/VISION.md");
  // Unknown arg shapes fall back to JSON of the args object.
  assert.equal(byId.get("t3")!.detail, JSON.stringify({ zeta: 1, alpha: "two" }));
  // Finished rows preview the result.
  assert.equal(byId.get("t1:finished")!.detail, "total 42");
  // Oversized previews truncate at 240 chars with an ellipsis.
  const truncated = byId.get("t4")!.detail!;
  assert.equal(truncated.length, 240);
  assert.equal(truncated.endsWith("…"), true);
});
