import assert from "node:assert/strict";
import test from "node:test";

import { create, toBinary, type DescMessage, type MessageShape } from "@bufbuild/protobuf";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
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
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionGenerationBumpedSchema,
  SessionModelChangedSchema,
  SessionRegisteredSchema,
  SessionSnapshotSchema,
  SessionStateEventSchema,
  SessionStateSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubscribeEventSchema,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
  type SubscribeEvent,
} from "@patchbay/contracts";
import fc from "fast-check";

import {
  PresentationProjection,
  emptyPresentationModel,
  fold,
  markUnreconciled,
  rendersLive,
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

  projection.replaceFromSnapshot(
    create(SessionSnapshotSchema, {
      authorityDomainId: DOMAIN,
      snapshotLsn: create(LsnSchema, { value: 0n }),
      sessions: [],
    }),
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
