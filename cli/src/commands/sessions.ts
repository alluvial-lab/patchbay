import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AuthorityDomainIdSchema,
  TargetScopeKind,
  TargetScopeSchema,
  SessionSnapshotSchema,
  type Session,
  type TargetScope,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";

export async function loadSessions(
  client: Pick<ControlClient, "loadSnapshot">,
  authorityDomainId: string,
): Promise<Session[]> {
  const response = await client.loadSnapshot({
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
  });
  if (!response.present) return [];
  if (response.snapshotPayload.length === 0) {
    throw new Error("core returned an empty snapshot payload");
  }

  const snapshot = fromBinary(SessionSnapshotSchema, response.snapshotPayload);
  if (snapshot.authorityDomainId?.value !== authorityDomainId) {
    throw new Error("core returned a snapshot from another authority domain");
  }
  if (response.eventId?.authorityDomainId?.value !== authorityDomainId) {
    throw new Error("core returned a snapshot event from another authority domain");
  }
  if (snapshot.snapshotLsn?.value !== response.eventId?.lsn?.value) {
    throw new Error("snapshot LSN does not match its response event LSN");
  }
  return snapshot.sessions;
}

export function resolveSession(sessions: readonly Session[], target: string): Session {
  if (!target) throw new Error("session target must not be empty");
  const live = sessions.filter((session) => !session.tombstoned);
  const matches = live.filter((session) =>
    [
      canonicalSessionIdentity(session),
      compactSessionIdentity(session),
      session.runtimeSessionId?.value,
      session.name || undefined,
    ].includes(target),
  );
  if (matches.length === 0) {
    throw new Error(`session target not found: ${target}`);
  }
  if (matches.length > 1) {
    const identities = matches.map(canonicalSessionIdentity).join(", ");
    throw new Error(`session target is ambiguous; use a stable identity: ${identities}`);
  }
  return matches[0]!;
}

export function sessionTargetScope(session: Session): TargetScope {
  const adapterId = required(session.adapterId?.value, "session adapter id");
  const runtimeSessionId = required(session.runtimeSessionId?.value, "runtime session id");
  const generation = session.sessionGeneration?.value;
  if (generation === undefined) throw new Error("session generation is missing");
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: { value: adapterId },
    deploymentScope: session.deploymentScope,
    runtimeSessionId: { value: runtimeSessionId },
    sessionGeneration: { value: generation },
  });
}

export function canonicalSessionIdentity(session: Session): string {
  const adapter = encodeURIComponent(required(session.adapterId?.value, "session adapter id"));
  const scope = encodeURIComponent(session.deploymentScope);
  const runtime = encodeURIComponent(required(session.runtimeSessionId?.value, "runtime session id"));
  const generation = session.sessionGeneration?.value;
  if (generation === undefined) throw new Error("session generation is missing");
  return `adapter=${adapter};scope=${scope};runtime=${runtime};generation=${generation}`;
}

function compactSessionIdentity(session: Session): string {
  const adapter = required(session.adapterId?.value, "session adapter id");
  const runtime = required(session.runtimeSessionId?.value, "runtime session id");
  const generation = session.sessionGeneration?.value;
  if (generation === undefined) throw new Error("session generation is missing");
  return `${adapter}/${session.deploymentScope}/${runtime}@${generation}`;
}

function required(value: string | undefined, name: string): string {
  if (!value) throw new Error(`${name} is missing`);
  return value;
}
