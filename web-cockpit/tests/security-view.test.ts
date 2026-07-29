import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

import { create } from "@bufbuild/protobuf";
import { AuthorityDomainIdSchema } from "@patchbay/contracts";
import { renderSecurityView } from "../src/ui/security-view.js";
import { emptyPresentationModel } from "../src/domain/model.js";

const domain = create(AuthorityDomainIdSchema, { value: "operator-domain" });

test("lockdown ritual requires arm, safe reason, and exact LOCKDOWN confirmation", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  let calls = 0;
  const view = renderSecurityView(dom.window.document, emptyPresentationModel(), domain, {
    async enterLockdown() { calls += 1; },
    async revokeAllSessions() {},
    async revokePrincipal() {},
    async revokeEndpoint() {},
    async revokeDevice() {},
    async revokeGrant() {},
  });
  dom.window.document.body.append(view);

  const arm = view.querySelector<HTMLButtonElement>(".security-hero .btn-danger")!;
  const dialogs = () => [...view.querySelectorAll<HTMLElement>("[role=dialog]")];
  arm.click();
  assert.equal(dialogs()[0]!.hidden, false);
  dialogs()[0]!.querySelector<HTMLButtonElement>(".btn-danger")!.click();
  assert.equal(dialogs()[0]!.hidden, true);
  assert.equal(dialogs()[1]!.hidden, false);

  const confirmation = view.querySelector<HTMLInputElement>("#lockdown-confirmation")!;
  const enter = dialogs()[1]!.querySelector<HTMLButtonElement>(".btn-danger")!;
  confirmation.value = "LOCKDOWN!";
  confirmation.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(enter.disabled, true);
  enter.click();
  assert.equal(calls, 0);

  confirmation.value = "LOCKDOWN";
  confirmation.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(enter.disabled, false);
  enter.click();
  await Promise.resolve();
  assert.equal(calls, 1);
  assert.equal([...view.querySelectorAll("button")].some((button) => /exit/i.test(button.textContent ?? "")), false);
});

test("active posture has inline read-only explanation and no exit affordance", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const model = emptyPresentationModel();
  model.lockdown = { active: true, reasonCode: "suspected_endpoint_compromise" };
  const view = renderSecurityView(dom.window.document, model, domain);
  assert.match(view.textContent ?? "", /Read-only during lockdown/);
  assert.equal(view.querySelector("button")?.textContent, "Lockdown active");
  assert.equal([...view.querySelectorAll("button")].some((button) => /exit/i.test(button.textContent ?? "")), false);
});
