import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AuthorityDomainIdSchema,
  TargetScopeKind,
  TargetScopeSchema,
  SessionSnapshotSchema,
  SnapshotViewKind,
  type Session,
  type TargetScope,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";

const MAX_U64 = (1n << 64n) - 1n;

export async function loadSessions(
  client: Pick<ControlClient, "loadSnapshot">,
  authorityDomainId: string,
): Promise<Session[]> {
  const response = await client.loadSnapshot({
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
    viewKind: SnapshotViewKind.SESSION,
  });
  if (!response.present) return [];
  if (response.viewKind !== SnapshotViewKind.SESSION) {
    throw new Error("core returned a non-session snapshot view");
  }
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

export function authorityDomainTarget(authorityDomainId: string): TargetScope {
  if (!authorityDomainId) throw new Error("authority domain id must not be empty");
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.AUTHORITY_DOMAIN,
    deploymentScope: authorityDomainId,
  });
}

export function parseCanonicalSessionTarget(value: string): TargetScope {
  const fields = new Map<string, string>();
  for (const part of value.split(";")) {
    const separator = part.indexOf("=");
    if (separator <= 0 || separator !== part.lastIndexOf("=")) throw new Error("malformed runtime-session target");
    const key = part.slice(0, separator);
    if (!["adapter", "scope", "runtime", "generation"].includes(key) || fields.has(key)) {
      throw new Error(`invalid runtime-session target key: ${key}`);
    }
    try {
      fields.set(key, decodeURIComponent(part.slice(separator + 1)));
    } catch {
      throw new Error("runtime-session target contains invalid percent-encoding");
    }
  }
  if (fields.size !== 4 || ["adapter", "scope", "runtime", "generation"].some((key) => !fields.get(key))) {
    throw new Error("runtime-session target must contain non-empty adapter, scope, runtime, and generation");
  }
  const generation = fields.get("generation")!;
  if (!/^\d+$/.test(generation)) throw new Error("runtime-session generation must be a non-negative integer");
  const generationValue = BigInt(generation);
  if (generationValue > MAX_U64) throw new Error("runtime-session generation exceeds uint64 range");
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: { value: fields.get("adapter")! },
    deploymentScope: fields.get("scope")!,
    runtimeSessionId: { value: fields.get("runtime")! },
    sessionGeneration: { value: generationValue },
  });
}

function required(value: string | undefined, name: string): string {
  if (!value) throw new Error(`${name} is missing`);
  return value;
}
