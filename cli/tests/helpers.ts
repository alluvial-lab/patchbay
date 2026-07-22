import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GenerationSchema,
  LoadSnapshotResponseSchema,
  LsnSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionSchema,
  SessionSnapshotSchema,
  SessionStateSchema,
  type Session,
} from "@patchbay/contracts";
import type { CliCredentials } from "../src/credentials.js";
import type { CliOutput } from "../src/main.js";

export const DOMAIN = "default";
export const BEARER_SECRET = "bearer-secret-must-not-appear";

export function credentials(): CliCredentials {
  return {
    version: 1,
    authorityDomainId: DOMAIN,
    operatorActorId: "operator-primary",
    sessionId: "session-core-issued",
    principal: {
      principalId: "principal-cli",
      secret: BEARER_SECRET,
      operatorActorId: "operator-primary",
      endpointId: "cli-endpoint",
      deviceId: "cli-device",
      endpointGeneration: "1",
    },
  };
}

export function session(): Session {
  return create(SessionSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
    adapterId: create(AdapterIdSchema, { value: "pi-adapter" }),
    deploymentScope: "machine-a",
    runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-1" }),
    sessionGeneration: create(GenerationSchema, { value: 3n }),
    name: "primary",
    state: create(SessionStateSchema, {
      connectivity: SessionConnectivityState.LIVE,
      activity: SessionActivityState.WORKING,
    }),
    lastAuthoritativeLsn: create(LsnSchema, { value: 7n }),
  });
}

export function snapshotResponse(sessions: Session[] = [session()]) {
  const snapshot = create(SessionSnapshotSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
    snapshotLsn: create(LsnSchema, { value: 7n }),
    sessions,
  });
  return create(LoadSnapshotResponseSchema, {
    present: true,
    eventId: create(EventIdSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
      lsn: create(LsnSchema, { value: 7n }),
    }),
    snapshotPayload: toBinary(SessionSnapshotSchema, snapshot),
  });
}

export function captureOutput(events?: string[]): CliOutput & { out: string[]; err: string[] } {
  const out: string[] = [];
  const err: string[] = [];
  return {
    out,
    err,
    stdout(line) {
      out.push(line);
      events?.push(`stdout:${line}`);
    },
    stderr(line) {
      err.push(line);
      events?.push(`stderr:${line}`);
    },
  };
}
