import { createHash } from "node:crypto";
import {
  TOKEN_COMMUNE_RESOURCE_KINDS,
  type TokenCommuneResourceKind,
} from "./resource_contract.js";

export interface SynthesizedResourceIdentity {
  adapterId: string;
  resourceKind: TokenCommuneResourceKind;
  resourceId: string;
}

export interface ResourceIdentitySynthesizer {
  readonly gatewayDeploymentKey: string;
  providerPool(provider: string): SynthesizedResourceIdentity;
  memberDraw(memberDisplayName: string, provider: string): SynthesizedResourceIdentity;
}

export function createCompositeLocalIdentitySynthesizer(input: {
  adapterId: string;
  gatewayBaseUrl: URL;
}): ResourceIdentitySynthesizer {
  if (!input.adapterId.trim()) throw new Error("adapter id is required");
  const canonicalUrl = canonicalGatewayUrl(input.gatewayBaseUrl);
  const deploymentHash = digest(canonicalUrl);
  const identity = (
    resourceKind: TokenCommuneResourceKind,
    resourceId: string,
  ): SynthesizedResourceIdentity => ({
    adapterId: input.adapterId,
    resourceKind,
    resourceId,
  });
  return Object.freeze({
    gatewayDeploymentKey: deploymentHash,
    providerPool(provider: string) {
      return identity(
        TOKEN_COMMUNE_RESOURCE_KINDS.providerPool,
        `local:provider-pool:${deploymentHash}:${safeSegment(provider, "provider")}`,
      );
    },
    memberDraw(memberDisplayName: string, provider: string) {
      const memberHash = digest(nonEmpty(memberDisplayName, "member display name"));
      return identity(
        TOKEN_COMMUNE_RESOURCE_KINDS.memberDraw,
        `local:member-draw:${deploymentHash}:${memberHash}:${safeSegment(provider, "provider")}`,
      );
    },
  });
}

export function canonicalGatewayUrl(input: URL): string {
  const value = new URL(input.href);
  if (!["http:", "https:"].includes(value.protocol)) throw new Error("gateway URL must use http or https");
  if (value.username || value.password || value.search || value.hash) {
    throw new Error("gateway URL must not contain credentials, query, or fragment");
  }
  value.hostname = value.hostname.toLowerCase();
  if ((value.protocol === "http:" && value.port === "80") || (value.protocol === "https:" && value.port === "443")) {
    value.port = "";
  }
  value.pathname = `/${value.pathname.split("/").filter(Boolean).map(decodeAndEncode).join("/")}`;
  if (value.pathname !== "/") value.pathname += "/";
  return value.href;
}

function digest(value: string): string {
  return createHash("sha256").update(value).digest("hex").slice(0, 24);
}

function safeSegment(value: string, name: string): string {
  return encodeURIComponent(nonEmpty(value, name));
}

function nonEmpty(value: string, name: string): string {
  if (!value.trim()) throw new Error(`${name} is required`);
  return value.trim();
}

function decodeAndEncode(value: string): string {
  try {
    return encodeURIComponent(decodeURIComponent(value));
  } catch {
    throw new Error("gateway URL path contains invalid percent encoding");
  }
}
