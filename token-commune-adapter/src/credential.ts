import { constants } from "node:fs";
import { lstat, open } from "node:fs/promises";

export interface GatewayCredential {
  apply(headers: Headers): void;
  redactionSecrets(): readonly string[];
  dispose(): void;
}

const CREDENTIAL_ERROR = "token-commune member credential is unavailable or unsafe";

export async function loadGatewayCredential(path: string): Promise<GatewayCredential> {
  if (!path) throw new Error(CREDENTIAL_ERROR);
  let handle;
  try {
    const before = await lstat(path);
    if (before.isSymbolicLink() || !before.isFile() || (before.mode & 0o777) !== 0o600) {
      throw new Error(CREDENTIAL_ERROR);
    }
    handle = await open(path, constants.O_RDONLY | noFollowFlag());
    const after = await handle.stat();
    if (
      !after.isFile() || (after.mode & 0o777) !== 0o600 ||
      before.dev !== after.dev || before.ino !== after.ino
    ) {
      throw new Error(CREDENTIAL_ERROR);
    }
    const raw = await handle.readFile({ encoding: "utf8" });
    const key = raw.endsWith("\r\n") ? raw.slice(0, -2) : raw.endsWith("\n") ? raw.slice(0, -1) : raw;
    if (!key || key !== key.trim() || /[\r\n]/.test(key)) throw new Error(CREDENTIAL_ERROR);
    return new FileGatewayCredential(key);
  } catch {
    throw new Error(CREDENTIAL_ERROR);
  } finally {
    await handle?.close().catch(() => undefined);
  }
}

class FileGatewayCredential implements GatewayCredential {
  #key: string | undefined;

  constructor(key: string) {
    this.#key = key;
  }

  apply(headers: Headers): void {
    const key = this.#key;
    if (!key) throw new Error("token-commune member credential has been disposed");
    headers.set("Authorization", `Bearer ${key}`);
  }

  redactionSecrets(): readonly string[] {
    if (!this.#key) return [];
    return [...new Set([
      this.#key,
      `Bearer ${this.#key}`,
      encodeURIComponent(this.#key),
      Buffer.from(this.#key).toString("base64"),
      JSON.stringify(this.#key),
    ])];
  }

  dispose(): void {
    // JavaScript strings cannot be reliably zeroized; dropping the final owned
    // reference is the strongest honest guarantee available here.
    this.#key = undefined;
  }
}

function noFollowFlag(): number {
  return typeof constants.O_NOFOLLOW === "number" ? constants.O_NOFOLLOW : 0;
}
