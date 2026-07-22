import { chmod, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { randomUUID } from "node:crypto";
import type { PrincipalCredential } from "@patchbay/contracts";

const STORE_VERSION = 1;

export interface StoredPrincipalCredential {
  principalId: string;
  secret: string;
  operatorActorId: string;
  endpointId: string;
  deviceId: string;
  endpointGeneration: string;
}

export interface CliCredentials {
  version: typeof STORE_VERSION;
  authorityDomainId: string;
  operatorActorId: string;
  sessionId: string;
  principal: StoredPrincipalCredential;
}

export interface CredentialReader {
  readRequired(): Promise<CliCredentials>;
}

export class CredentialStore implements CredentialReader {
  constructor(readonly path: string) {
    if (!path) throw new Error("credential store path must not be empty");
  }

  async read(): Promise<CliCredentials | null> {
    let serialized: string;
    try {
      serialized = await readFile(this.path, "utf8");
    } catch (error) {
      if (isNodeError(error) && error.code === "ENOENT") return null;
      throw error;
    }

    let candidate: unknown;
    try {
      candidate = JSON.parse(serialized);
    } catch {
      throw new Error(`credential store is not valid JSON: ${this.path}`);
    }
    return validateCredentials(candidate, this.path);
  }

  async readRequired(): Promise<CliCredentials> {
    const credentials = await this.read();
    if (!credentials) {
      throw new Error(`CLI credentials not found at ${this.path}; run patchbay-cli login`);
    }
    return credentials;
  }

  async write(credentials: CliCredentials): Promise<void> {
    const validated = validateCredentials(credentials, this.path);
    const directory = dirname(this.path);
    const createdDirectory = await mkdir(directory, { recursive: true, mode: 0o700 });
    if (createdDirectory !== undefined) {
      await chmod(directory, 0o700);
    }

    const temporary = `${this.path}.${process.pid}.${randomUUID()}.tmp`;
    try {
      await writeFile(temporary, `${JSON.stringify(validated, null, 2)}\n`, {
        encoding: "utf8",
        mode: 0o600,
        flag: "wx",
      });
      await rename(temporary, this.path);
      await chmod(this.path, 0o600);
    } catch (error) {
      await rm(temporary, { force: true }).catch(() => undefined);
      throw error;
    }
  }

  async clear(): Promise<void> {
    await rm(this.path, { force: true });
  }
}

export function credentialsFromRpc(
  authorityDomainId: string,
  sessionId: string | undefined,
  principal: PrincipalCredential | undefined,
): CliCredentials {
  if (!authorityDomainId) throw new Error("authority domain id is missing");
  if (!sessionId) throw new Error("core returned no operator session id");
  if (
    !principal?.principalId ||
    !principal.secret ||
    !principal.operatorActorId?.value ||
    !principal.endpointId?.value ||
    !principal.deviceId?.value ||
    principal.endpointGeneration === undefined
  ) {
    throw new Error("core returned an incomplete control-surface principal credential");
  }

  return validateCredentials(
    {
      version: STORE_VERSION,
      authorityDomainId,
      operatorActorId: principal.operatorActorId.value,
      sessionId,
      principal: {
        principalId: principal.principalId,
        secret: principal.secret,
        operatorActorId: principal.operatorActorId.value,
        endpointId: principal.endpointId.value,
        deviceId: principal.deviceId.value,
        endpointGeneration: principal.endpointGeneration.value.toString(),
      },
    },
    "core enrollment result",
  );
}

function validateCredentials(candidate: unknown, source: string): CliCredentials {
  if (!isRecord(candidate) || candidate.version !== STORE_VERSION) {
    throw new Error(`unsupported credential store version in ${source}`);
  }
  const principal = candidate.principal;
  if (!isRecord(principal)) throw new Error(`credential store principal is missing in ${source}`);

  const authorityDomainId = requiredString(candidate.authorityDomainId, "authorityDomainId", source);
  const operatorActorId = requiredString(candidate.operatorActorId, "operatorActorId", source);
  const sessionId = requiredString(candidate.sessionId, "sessionId", source);
  const storedPrincipal: StoredPrincipalCredential = {
    principalId: requiredString(principal.principalId, "principal.principalId", source),
    secret: requiredString(principal.secret, "principal.secret", source),
    operatorActorId: requiredString(principal.operatorActorId, "principal.operatorActorId", source),
    endpointId: requiredString(principal.endpointId, "principal.endpointId", source),
    deviceId: requiredString(principal.deviceId, "principal.deviceId", source),
    endpointGeneration: requiredGeneration(principal.endpointGeneration, source),
  };
  if (storedPrincipal.operatorActorId !== operatorActorId) {
    throw new Error(`credential store actor/principal binding is inconsistent in ${source}`);
  }

  return {
    version: STORE_VERSION,
    authorityDomainId,
    operatorActorId,
    sessionId,
    principal: storedPrincipal,
  };
}

function requiredString(value: unknown, field: string, source: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`credential store ${field} is missing in ${source}`);
  }
  return value;
}

function requiredGeneration(value: unknown, source: string): string {
  const generation = requiredString(value, "principal.endpointGeneration", source);
  if (!/^\d+$/.test(generation)) {
    throw new Error(`credential store principal.endpointGeneration is invalid in ${source}`);
  }
  return generation;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}
