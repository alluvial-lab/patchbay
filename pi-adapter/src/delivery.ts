import { fromBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  OperationKind,
  PayloadContentType,
  type Operation,
} from "@patchbay/contracts";
import type { PiSession } from "./pi_session.js";

const decoder = new TextDecoder();

export class UnsupportedCommandError extends Error {
  readonly failureCode = "unsupported_command";
}

export interface DeliveryOutcome {
  sessionGenerationChanged?: boolean;
  value?: unknown;
}

/** Single registry-derived OperationKind dispatch point for Pi actions. */
export class DeliveryTranslator {
  async deliver(operation: Operation, session: PiSession): Promise<DeliveryOutcome> {
    switch (operation.kind) {
      case OperationKind.INSTRUCT:
        await session.prompt(requiredText(operation));
        return {};
      case OperationKind.CANCEL:
      case OperationKind.INTERRUPT:
        await session.cancel();
        return {};
      case OperationKind.QUERY:
        return { value: this.#query(operation, session) };
      case OperationKind.RECONFIGURE:
        await this.#reconfigure(operation, session);
        return {};
      case OperationKind.SESSION_MANAGEMENT:
        return this.#manageSession(operation, session);
      case OperationKind.SPAWN:
        throw new UnsupportedCommandError("Pi spawn is unsupported in v0.1.0");
      case OperationKind.APPROVAL_RESPONSE: {
        const payload = approvalPayload(operation);
        if (payload.decision === ApprovalDecision.APPROVED) {
          await session.resolveApproval(operation, true);
          return {};
        }
        if (payload.decision === ApprovalDecision.DENIED) {
          await session.resolveApproval(operation, false);
          return {};
        }
        throw new UnsupportedCommandError(
          `approval decision ${payload.decision} is not deliverable in v0.1.0`,
        );
      }
      case OperationKind.ELICITATION_RESPONSE:
        throw new UnsupportedCommandError(
          "question Elicitation delivery is an explicit minimal-slice follow-on",
        );
      case OperationKind.ATTACH:
      case OperationKind.RESERVED_ADAPTER_UTILITY_EXEC:
      case OperationKind.RESERVED_AGENT_SEND:
      case OperationKind.UNSPECIFIED:
        throw new UnsupportedCommandError(`Pi cannot deliver OperationKind ${operation.kind}`);
    }
  }

  #query(operation: Operation, session: PiSession): unknown {
    const payload = objectPayload(operation);
    switch (payload["action"] ?? "state") {
      case "state":
        return session.getState();
      case "entries":
        return session.getEntries(stringField(payload, "since", false));
      case "models":
        return session.getAvailableModels();
      default:
        throw new UnsupportedCommandError(`unknown Pi query action: ${String(payload["action"])}`);
    }
  }

  async #reconfigure(operation: Operation, session: PiSession): Promise<void> {
    const payload = objectPayload(operation);
    const action = stringField(payload, "action");
    if (action === "model") {
      await session.setModel(stringField(payload, "provider"), stringField(payload, "modelId"));
      return;
    }
    if (action === "thinking") {
      await session.setThinkingLevel(
        stringField(payload, "level") as Parameters<PiSession["setThinkingLevel"]>[0],
      );
      return;
    }
    throw new UnsupportedCommandError(`unknown Pi reconfigure action: ${action}`);
  }

  async #manageSession(
    operation: Operation,
    session: PiSession,
  ): Promise<DeliveryOutcome> {
    const payload = objectPayload(operation);
    const action = stringField(payload, "action");
    if (action === "new") {
      await session.newSession();
      return { sessionGenerationChanged: true };
    }
    if (action === "compact") {
      await session.compact(stringField(payload, "instructions", false));
      return {};
    }
    throw new UnsupportedCommandError(`unknown Pi session action: ${action}`);
  }
}

function approvalPayload(operation: Operation) {
  const envelope = operation.payload;
  if (!envelope || envelope.contentType !== PayloadContentType.PROTOBUF) {
    throw new UnsupportedCommandError(
      "approval response payload content_type must be PAYLOAD_CONTENT_TYPE_PROTOBUF",
    );
  }
  try {
    return fromBinary(ApprovalResponsePayloadSchema, envelope.payload);
  } catch (error) {
    throw new UnsupportedCommandError(
      `cannot decode ApprovalResponsePayload: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function requiredText(operation: Operation): string {
  const text = decoder.decode(operation.payload?.payload ?? new Uint8Array());
  if (!text) throw new Error("instruct payload is empty");
  if (text.trimStart().startsWith("{")) {
    const parsed = JSON.parse(text) as unknown;
    const payload = asRecord(parsed);
    return stringField(payload, "text");
  }
  return text;
}

function objectPayload(operation: Operation): Record<string, unknown> {
  const text = decoder.decode(operation.payload?.payload ?? new Uint8Array());
  if (!text) return {};
  return asRecord(JSON.parse(text));
}

function asRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("operation payload must be a JSON object");
  }
  return value as Record<string, unknown>;
}

function stringField(value: Record<string, unknown>, field: string): string;
function stringField(
  value: Record<string, unknown>,
  field: string,
  required: false,
): string | undefined;
function stringField(
  value: Record<string, unknown>,
  field: string,
  required = true,
): string | undefined {
  const found = value[field];
  if (typeof found === "string" && found.length > 0) return found;
  if (!required) return undefined;
  throw new Error(`operation payload is missing ${field}`);
}
