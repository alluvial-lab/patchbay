import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AuthorityDomainIdSchema,
  OperationKind,
  PayloadContentType,
  SessionActivityState,
  SessionConnectivityState,
} from "@patchbay/contracts";
import { JSDOM } from "jsdom";

import { buildInstructOperation, startCockpit } from "../src/main.js";
import type { SessionView } from "../src/domain/model.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });

test("browser entry composes protocol client, projection, reconciler, and shell", async () => {
  const dom = new JSDOM(`<!doctype html>
    <meta name="patchbay-authority-domain" content="operator-domain">
    <main data-patchbay-cockpit></main>`);
  const fetcher = (async (input: RequestInfo | URL) => {
    assert.equal(String(input), "/csrf-token");
    return new Response(JSON.stringify({ csrfToken: "csrf-proof" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as typeof globalThis.fetch;

  const app = await startCockpit({
    document: dom.window.document,
    fetch: fetcher,
    startSubscription: false,
    isMobile: () => false,
  });

  assert.ok(app.protocol.client);
  assert.ok(app.projection);
  assert.ok(app.reconciler);
  assert.equal(dom.window.document.querySelectorAll(".cockpit").length, 1);
  assert.equal(app.shell.element.isConnected, true);
  app.stop();
});

test("composition submission builder emits a boundary-valid instruct Operation", () => {
  const operation = buildInstructOperation(
    DOMAIN,
    session(),
    "Run the verification suite",
    { commandId: "command-browser-1", idempotencyKey: "idem-browser-1" },
  );

  assert.equal(operation.kind, OperationKind.INSTRUCT);
  assert.equal(operation.commandId?.value, "command-browser-1");
  assert.equal(operation.authorityDomainId?.value, DOMAIN.value);
  assert.ok(operation.sender, "the web server replaces this untrusted sender envelope");
  assert.equal(operation.targetScope?.adapterId?.value, "pi");
  assert.equal(operation.targetScope?.runtimeSessionId?.value, "session-1");
  assert.equal(operation.targetScope?.sessionGeneration?.value, 1n);
  assert.equal(operation.idempotencyKey, "idem-browser-1");
  assert.equal(operation.payload?.contentType, PayloadContentType.TEXT_UTF8);
  assert.equal(new TextDecoder().decode(operation.payload?.payload), "Run the verification suite");
});

test("browser build emits a servable HTML entry and bundled module", async () => {
  const html = await readFile(new URL("../index.html", import.meta.url), "utf8");
  const bundle = await readFile(new URL("../assets/cockpit.js", import.meta.url), "utf8");

  assert.match(html, /data-patchbay-cockpit/);
  assert.match(html, /\/assets\/cockpit\.js/);
  assert.match(bundle, /startCockpit/);
});

function session(): SessionView {
  return {
    identity: {
      adapterId: "pi",
      deploymentScope: "laptop",
      runtimeSessionId: "session-1",
      generation: 1n,
    },
    label: { project: "patchbay", name: "core" },
    connectivity: SessionConnectivityState.LIVE,
    activity: SessionActivityState.IDLE,
    needsYou: false,
    lastLsn: 1n,
    tombstoned: false,
    reconciled: true,
  };
}
