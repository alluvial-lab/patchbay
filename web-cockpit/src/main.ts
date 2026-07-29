import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  ActorEndpointRefSchema,
  AuthorityDomainIdSchema,
  EnterSecurityLockdownRequestSchema,
  CommandIdSchema,
  LocalSubmissionState,
  OperationKind,
  OperationSchema,
  QueryDiagnosticsRequestSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SubmitRequestSchema,
  SubmissionOutcome,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
  type AuthorityDomainId,
  type Operation,
} from "@patchbay/contracts";

import {
  lockdownViewFromState,
  PresentationProjection,
  type SessionIdentity,
  type SessionView,
} from "./domain/model.js";
import {
  createProtocolClient,
  CsrfTokenRequestError,
  fetchCsrfToken,
  type ProtocolClient,
} from "./domain/protocol-client.js";
import { Reconciler } from "./domain/reconcile.js";
import {
  buildAdapterStatusQueryOperation,
  clearAdapterStatus,
  mergeAdapterStatusResult,
} from "./domain/adapter-diagnostics.js";
import { createMobileElicitationSheet } from "./ui/elicitation.js";
import { waitForOperatorLogin } from "./ui/login.js";
import { createMarkdownRenderer } from "./ui/markdown.js";
import { createRenderCoalescer } from "./ui/render-coalescer.js";
import { createCockpitShell, type CockpitShell } from "./ui/shell.js";
import type { SubmissionFeedback } from "./ui/session-detail.js";

export interface StartCockpitOptions {
  document?: Document;
  mount?: HTMLElement;
  authorityDomainId?: AuthorityDomainId;
  fetch?: typeof globalThis.fetch;
  baseUrl?: string;
  idFactory?: () => string;
  startSubscription?: boolean;
  isMobile?: () => boolean;
}

export interface CockpitApp {
  readonly protocol: ProtocolClient;
  readonly projection: PresentationProjection;
  readonly reconciler: Reconciler;
  readonly shell: CockpitShell;
  stop(): void;
}

/** Browser composition root: transport + CSRF + projection + reconciler + live shell. */
export async function startCockpit(options: StartCockpitOptions = {}): Promise<CockpitApp> {
  const document = options.document ?? globalThis.document;
  const mount = options.mount ?? document.querySelector<HTMLElement>("[data-patchbay-cockpit]");
  if (!mount) throw new Error("cockpit mount element is missing");
  const authorityDomainId = options.authorityDomainId ?? authorityDomainFromDocument(document);
  // Bind the global fetch: passing the bare function through as a callback
  // would call it with the wrong receiver, and browsers require `window` as
  // `this` ("'fetch' called on an object that does not implement interface
  // Window"). Tests inject their own fetch, so this binding is browser-only.
  const fetcher = options.fetch ?? globalThis.fetch.bind(globalThis);
  let csrfToken: string;
  try {
    csrfToken = await fetchCsrfToken(fetcher);
  } catch (error) {
    // 401 = no/unknown session; 403 = known-but-inactive session (expired or
    // revoked). Both mean the operator must log in — a session expiring is
    // normal, never a startup failure.
    if (!(error instanceof CsrfTokenRequestError) || (error.status !== 401 && error.status !== 403)) throw error;
    await waitForOperatorLogin(document, mount, { fetch: fetcher });
    csrfToken = await fetchCsrfToken(fetcher);
  }
  const protocol = createProtocolClient({
    baseUrl: options.baseUrl,
    fetch: fetcher,
    csrfToken: () => csrfToken,
  });
  return composeCockpit(document, mount, authorityDomainId, protocol, {
    idFactory: options.idFactory,
    startSubscription: options.startSubscription,
    isMobile: options.isMobile,
    fetcher,
    refreshCsrfToken: async () => {
      csrfToken = await fetchCsrfToken(fetcher);
      return csrfToken;
    },
  });
}

interface ComposeOptions {
  idFactory?: () => string;
  startSubscription?: boolean;
  isMobile?: () => boolean;
  fetcher: typeof globalThis.fetch;
  refreshCsrfToken: () => Promise<string>;
  /** Frame scheduler for render coalescing; defaults to requestAnimationFrame. */
  scheduleFrame?: (callback: () => void) => void;
}

function composeCockpit(
  document: Document,
  mount: HTMLElement,
  authorityDomainId: AuthorityDomainId,
  protocol: ProtocolClient,
  options: ComposeOptions,
): CockpitApp {
  const projection = new PresentationProjection();
  const abort = new AbortController();
  let shell!: CockpitShell;
  const reconciler = new Reconciler(protocol.client, projection, {
    // Reconciliation is a domain signal, not a shell-render edge. A snapshot
    // can briefly make model.reconciled=false without producing a render, so
    // diagnostics refreshes must subscribe to the completed reconnect itself.
    onReconciliationComplete() {
      if (!shell) return;
      shell.update(projection.model);
      const selectedKey = shell.selectedSessionKey;
      const selected = selectedKey
        ? projection.model.sessions.get(selectedKey)
        : undefined;
      void queryAdapterStatus(selected, "reconcile");
    },
  });
  const idFactory = options.idFactory ?? (() => globalThis.crypto.randomUUID());
  let submission: SubmissionFeedback | undefined;
  const inFlightDiagnostics = new Set<string>();
  let diagnosticRequestSequence = 0;

  async function queryAdapterStatus(session: SessionView | undefined, reason: string): Promise<void> {
    if (projection.model.lockdown.active || projection.model.lockdown.submitting) return;
    const adapterId = session?.identity.adapterId;
    if (!adapterId) return;
    const key = `${adapterId}:${reason}`;
    if (inFlightDiagnostics.has(key)) return;
    inFlightDiagnostics.add(key);
    try {
      const suffix = `${idFactory()}-${++diagnosticRequestSequence}`;
      const operation = buildAdapterStatusQueryOperation(authorityDomainId, adapterId, {
        commandId: `diagnostics-${suffix}`,
        idempotencyKey: `diagnostics-${suffix}`,
      });
      const response = await protocol.client.queryDiagnostics(
        create(QueryDiagnosticsRequestSchema, { operation }),
      );
      // Rejected/failed query responses are ordinary protocol values. The
      // merge validates accepted+completed+adapter-result and clears the
      // cached status while retaining newer live diagnostics.
      projection.model = mergeAdapterStatusResult(projection.model, response, adapterId);
      shell.update(projection.model);
    } catch {
      projection.model = clearAdapterStatus(projection.model, adapterId);
      shell.update(projection.model);
    } finally {
      inFlightDiagnostics.delete(key);
    }
  }

  async function submit(operation: Operation): Promise<void> {
    if (projection.model.lockdown.active || projection.model.lockdown.submitting) {
      submission = {
        state: LocalSubmissionState.SUBMIT_FAILED,
        error: projection.model.lockdown.active
          ? "Read-only during lockdown. New Operations are rejected until trusted bootstrap exit."
          : "Lockdown decision pending. Operations are disabled until the core confirms or denies entry.",
      };
      shell.update(projection.model);
      return;
    }
    submission = { state: LocalSubmissionState.SUBMITTING };
    shell.update(projection.model);
    try {
      const result = await protocol.client.submit(create(SubmitRequestSchema, { operation }));
      submission = {
        state:
          result.outcome === SubmissionOutcome.FAILED
            ? LocalSubmissionState.SUBMIT_FAILED
            : result.outcome === SubmissionOutcome.UNKNOWN
              ? LocalSubmissionState.UNKNOWN
              : LocalSubmissionState.DRAFT,
        result,
      };
      shell.update(projection.model);
    } catch (error) {
      submission = {
        state: LocalSubmissionState.UNKNOWN,
        error: error instanceof Error ? error.message : String(error),
      };
      shell.update(projection.model);
    }
  }

  const nextIds = () => {
    const suffix = idFactory();
    return { commandId: `command-${suffix}`, idempotencyKey: `idempotency-${suffix}` };
  };
  const mobileSheet = createMobileElicitationSheet(document, { isMobile: options.isMobile });
  const securityActions = {
    async enterLockdown(reasonCode: string): Promise<void> {
      // This is deliberately a local, non-authoritative presentation state.
      // Controls are disabled while the RPC is pending, but the cockpit must
      // not claim containment until the core confirms it.
      const priorLockdown = projection.model.lockdown;
      projection.model = {
        ...projection.model,
        lockdown: { ...priorLockdown, submitting: true },
      };
      shell.update(projection.model);
      try {
        const result = await protocol.client.enterSecurityLockdown(
          create(EnterSecurityLockdownRequestSchema, {
            authorityDomainId,
            reasonCode,
          }),
        );
        if (!result.lockdown) throw new Error("lockdown confirmation is missing");
        projection.model = {
          ...projection.model,
          lockdown: lockdownViewFromState(result.lockdown),
        };
        shell.update(projection.model);
      } catch (error) {
        const denied = error instanceof ConnectError
          && (error.code === Code.PermissionDenied || error.code === Code.FailedPrecondition);
        // A confirmed denial restores the posture that was visible before the
        // attempt. An unknown transport outcome also restores that posture,
        // then forces a fresh login/reconciliation rather than claiming entry.
        projection.model = {
          ...projection.model,
          lockdown: { ...priorLockdown, submitting: false },
        };
        shell.update(projection.model);
        if (denied) return;

        projection.markUnreconciled();
        shell.update(projection.model);
        try {
          await reconciler.reconcileNow(authorityDomainId);
          shell.update(projection.model);
        } catch {
          // If the session itself expired, force the pre-protocol login path.
          // The shell is reattached after login; until then no lockdown claim
          // is rendered and the projection remains unreconciled.
          await waitForOperatorLogin(document, mount, { fetch: options.fetcher });
          await options.refreshCsrfToken();
          mount.replaceChildren(shell.element);
          await reconciler.reconcileNow(authorityDomainId);
          shell.update(projection.model);
        }
      }
    },
    async revokeCurrentSession(): Promise<void> {
      await protocol.client.revokeOperatorSession({});
    },
    async revokeAllSessions(): Promise<void> {
      await protocol.client.revokeAllOperatorSessions({ reasonCode: "operator_security_action" });
    },
    async revokePrincipal(principalId: string): Promise<void> {
      await protocol.client.revokeControlSurfacePrincipal({ principalId, reasonCode: "operator_security_action" });
    },
    async revokeEndpoint(endpointId: string): Promise<void> {
      await protocol.client.revokeControlSurfaceEndpoint({
        target: { case: "endpointId", value: { value: endpointId } },
        reasonCode: "operator_security_action",
      });
    },
    async revokeDevice(deviceId: string): Promise<void> {
      await protocol.client.revokeControlSurfaceEndpoint({
        target: { case: "deviceId", value: { value: deviceId } },
        reasonCode: "operator_security_action",
      });
    },
    async revokeGrant(grantId: string): Promise<void> {
      await protocol.client.revokeGrant({
        authorityDomainId,
        grantId: { value: grantId },
        reason: "operator_security_action",
      });
    },
  };
  shell = createCockpitShell(document, projection.model, {
    markdown: createMarkdownRenderer(document.defaultView as unknown as Window),
    authorityDomainId,
    securityActions,
    isMobile: options.isMobile,
    submission: () => submission,
    onSelectionChange(session, reason) {
      void queryAdapterStatus(session, reason);
    },
    actions: {
      send(session, text) {
        return submit(buildInstructOperation(authorityDomainId, session, text, nextIds()));
      },
      cancel(command) {
        if (!command.target) throw new Error("cancel target identity is missing");
        return submit(buildCommandAction(authorityDomainId, command.target, command.id, OperationKind.CANCEL, nextIds()));
      },
      interrupt(command) {
        if (!command.target) throw new Error("interrupt target identity is missing");
        return submit(buildCommandAction(authorityDomainId, command.target, command.id, OperationKind.INTERRUPT, nextIds()));
      },
    },
    elicitation: {
      mobileSheet,
      operationContext(elicitation) {
        if (!elicitation.target) throw new Error("Elicitation target identity is missing");
        const ids = nextIds();
        return {
          authorityDomainId,
          targetScope: targetScope(elicitation.target),
          commandId: ids.commandId,
          idempotencyKey: ids.idempotencyKey,
        };
      },
      submit,
      reportError(error) {
        submission = {
          state: LocalSubmissionState.SUBMIT_FAILED,
          error: error instanceof Error ? error.message : String(error),
        };
        shell.update(projection.model);
      },
    },
  });
  mount.replaceChildren(shell.element);

  // Security inventory is a separate redacted snapshot projection. Load it
  // at startup (and again on stream reconnect) instead of inferring endpoint,
  // device, or grant rows from session events or audit prose.
  void reconciler.loadSecuritySnapshot(authorityDomainId)
    .then(() => shell.update(projection.model))
    .catch(() => {
      // The session cockpit remains readable if the dedicated inventory read
      // is temporarily unavailable; the next reconnect retries it.
    });

  if (options.startSubscription !== false) {
    // Fold every event as it arrives, but render at most once per frame: a
    // streaming turn emits thousands of deltas, and one synchronous full
    // timeline re-render per event saturates the main thread for the whole
    // turn (found in live dogfooding: tool rows painted, text never did).
    const frame =
      options.scheduleFrame ??
      (typeof globalThis.requestAnimationFrame === "function"
        ? globalThis.requestAnimationFrame.bind(globalThis)
        : (callback: () => void) => setTimeout(callback, 16));
    const coalescer = createRenderCoalescer(frame, () => shell.update(projection.model));
    void (async () => {
      for await (const _ of reconciler.subscribe(authorityDomainId, abort.signal)) {
        coalescer.notify();
      }
      coalescer.flush();
    })();
  }

  return {
    protocol,
    projection,
    reconciler,
    shell,
    stop() {
      abort.abort();
      shell.destroy();
    },
  };
}

export function buildInstructOperation(
  authorityDomainId: AuthorityDomainId,
  session: SessionView,
  text: string,
  ids: { commandId: string; idempotencyKey: string },
): Operation {
  if (!text.trim()) throw new Error("instruction text is empty");
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: ids.commandId }),
    authorityDomainId,
    // The web server replaces this untrusted browser claim from its verified session.
    sender: create(ActorEndpointRefSchema, {}),
    kind: OperationKind.INSTRUCT,
    targetScope: targetScope(session.identity),
    idempotencyKey: ids.idempotencyKey,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.TEXT_UTF8,
      payload: new TextEncoder().encode(text),
    }),
  });
}

function buildCommandAction(
  authorityDomainId: AuthorityDomainId,
  target: SessionIdentity,
  targetCommandId: string,
  kind: OperationKind.CANCEL | OperationKind.INTERRUPT,
  ids: { commandId: string; idempotencyKey: string },
): Operation {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: ids.commandId }),
    authorityDomainId,
    sender: create(ActorEndpointRefSchema, {}),
    kind,
    targetScope: targetScope(target),
    idempotencyKey: ids.idempotencyKey,
    correlations: [
      create(TypedCorrelationSchema, {
        ref: {
          case: "commandId",
          value: create(CommandIdSchema, { value: targetCommandId }),
        },
      }),
    ],
  });
}

function targetScope(identity: SessionIdentity) {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: { value: identity.adapterId },
    deploymentScope: identity.deploymentScope,
    runtimeSessionId: { value: identity.runtimeSessionId },
    sessionGeneration: { value: identity.generation },
  });
}

function authorityDomainFromDocument(document: Document): AuthorityDomainId {
  const value = document.querySelector<HTMLMetaElement>('meta[name="patchbay-authority-domain"]')?.content;
  if (!value) throw new Error("patchbay authority domain metadata is missing");
  return create(AuthorityDomainIdSchema, { value });
}

function renderStartupFailure(document: Document, error: unknown): void {
  const mount = document.querySelector<HTMLElement>("[data-patchbay-cockpit]");
  if (!mount) return;
  const banner = document.createElement("div");
  banner.className = "failure-banner";
  const term = document.createElement("span");
  term.className = "failure-banner__term";
  term.textContent = "cockpit_startup_failed";
  const message = document.createElement("p");
  message.className = "failure-banner__message";
  message.textContent = error instanceof Error ? error.message : String(error);
  banner.append(term, message);
  mount.replaceChildren(banner);
}

if (typeof document !== "undefined" && document.querySelector("[data-patchbay-cockpit]")) {
  startCockpit().catch((error: unknown) => renderStartupFailure(document, error));
}
