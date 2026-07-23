import assert from "node:assert/strict";
import test from "node:test";

import { stringifyContent } from "../src/transcript_projection.js";

// Regression for the live-use "garbled text" report: models that emit
// thinking/reasoning blocks (Kimi/GLM with thinking enabled) had their
// chain-of-thought concatenated into the rendered assistant message. Thinking
// surfaces via the activity-detail channel (turn_started → "thinking") and is
// never message content.

test("stringifyContent renders text blocks only, skipping thinking blocks", () => {
  const content = [
    { type: "thinking", thinking: "We should ignore? Means no action needed..." },
    { type: "text", text: "Ignored." },
  ];
  assert.equal(stringifyContent(content), "Ignored.");
});

test("stringifyContent passes through plain strings", () => {
  assert.equal(stringifyContent("hello"), "hello");
});

test("stringifyContent returns empty for thinking-only content", () => {
  const content = [{ type: "thinking", thinking: "internal reasoning" }];
  assert.equal(stringifyContent(content), "");
});

test("stringifyContent joins multiple text blocks", () => {
  const content = [
    { type: "text", text: "First. " },
    { type: "thinking", thinking: "reasoning in between" },
    { type: "text", text: "Second." },
  ];
  assert.equal(stringifyContent(content), "First. Second.");
});
