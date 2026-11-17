// Tests for the pull request description check: the template as GitHub fills it in fails, a complete description
// passes, and each rule fails on its own. Run with `node --test .github/scripts`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { checkDescription } from "./check-pull-request.mjs";

const template = readFileSync(new URL("../pull_request_template.md", import.meta.url), "utf8");

const complete = `
## Overview
Open strings light on the fretboard.
## Details
FretGeometry::cell now returns the open column.
## Test coverage
cells_are_where_strings_and_frets_cross fails with the change reverted.
## Evidence
https://github.com/user-attachments/assets/0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b
## Performance
added delay over 118 of 118 notes: median 3.08 ms, p95 5.84 ms (Core Ultra 9 285).
## Platforms
- [x] Windows
- [ ] macOS
## AI disclosure
None.
## Checklist
- [x] I read CONTRIBUTING.md and AGENTS.md.
- [x] validate passes.
`;

const rust = ["crates/riffology-fretboard/src/geometry.rs"];

test("the untouched template fails with every section named", () => {
  const p = checkDescription(template, rust);
  for (const name of ["Overview", "Details", "Test coverage", "Evidence", "Performance", "AI disclosure"]) {
    assert.ok(p.some((x) => x.includes(`"${name}"`)), `${name}: ${p.join(" | ")}`);
  }
  assert.ok(p.some((x) => x.includes("platform")));
  assert.ok(p.some((x) => x.includes("checklist box")));
});

test("a complete description passes", () => {
  assert.deepEqual(checkDescription(complete, rust), []);
});

test("a description with no video fails", () => {
  const p = checkDescription(complete.replace(/https:\/\/github.com\/user-attachments\S+/, "see the screenshot"), rust);
  assert.ok(p.some((x) => x.includes("links no video")));
});

test("a Rust change with no numbers fails, a docs change with an explained line passes", () => {
  const noNumbers = complete.replace(/added delay[^\n]+/, "It is fast.");
  assert.ok(checkDescription(noNumbers, rust).some((x) => x.includes("Performance")));
  assert.deepEqual(checkDescription(noNumbers, ["docs/latency.md"]), []);
  assert.deepEqual(checkDescription(complete.replace(/added delay[^\n]+/, "No timing change: documentation only."), rust), []);
});

test("an unticked checklist box and a missing heading fail", () => {
  assert.ok(checkDescription(complete.replace("- [x] validate", "- [ ] validate"), rust).some((x) => x.includes("1 checklist box is")));
  assert.ok(checkDescription(complete.replace("## Details", "## Notes"), rust).some((x) => x.includes(`"Details" heading`)));
});
