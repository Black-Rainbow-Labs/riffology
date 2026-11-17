// Checks that a pull request's description fills in .github/pull_request_template.md.
//
// Run by .github/workflows/pull-request-description.yml with GITHUB_TOKEN and the event payload. It reads the
// description and the changed file names through the API, never the pull request's code, and posts or updates one
// comment that lists what is missing. `checkDescription` is exported so its rules are tested without GitHub
// (check-pull-request.test.mjs).

import { readFileSync } from "node:fs";

/** The sections every description needs, in template order. */
export const SECTIONS = ["Overview", "Details", "Test coverage", "Evidence", "Performance", "Platforms", "AI disclosure", "Checklist"];

/** The marker that finds this script's own comment on a pull request. */
const MARKER = "<!-- riffology-pr-check -->";

/**
 * Removes HTML comments, which hold the template's guidance and do not count as the author's words.
 * @param text - the description
 */
export function stripComments(text) {
  return text.replace(/<!--[\s\S]*?-->/g, "");
}

/**
 * Splits a description into its `## ` sections.
 * @param text - the description with comments removed
 */
export function sectionsOf(text) {
  const out = new Map();
  let name = null;
  for (const line of text.split(/\r?\n/)) {
    const h = line.match(/^##\s+(.+?)\s*$/);
    if (h) {
      name = h[1];
      out.set(name, "");
    } else if (name) {
      out.set(name, out.get(name) + line + "\n");
    }
  }
  return out;
}

/**
 * Whether a section has words of the author's own: anything left once checkboxes are removed.
 * @param body - the section text
 */
function hasWords(body) {
  return body.replace(/^\s*-\s*\[[ xX]\].*$/gm, "").trim().length > 0;
}

/**
 * Whether the Evidence section links a video: a .mp4, .mov or .webm file, or a GitHub upload.
 * @param body - the Evidence section
 */
export function hasVideo(body) {
  return /https?:\/\/\S+\.(mp4|mov|webm)\b/i.test(body) || /https:\/\/github\.com\/user-attachments\/assets\/[0-9a-f-]+/i.test(body);
}

/**
 * Whether the Performance section has a measured number with a unit, or an explained "No timing change:" line.
 * @param body - the Performance section
 */
export function hasNumbers(body) {
  return /\d+(\.\d+)?\s*(ms|µs|us|fps|%|frames|F1)\b/i.test(body) || /\bF1\s+\d/i.test(body) || /^\s*No timing change:\s*\S+/im.test(body);
}

/**
 * Lists what a description is missing.
 * @param description - the pull request body
 * @param files - the changed file names
 */
export function checkDescription(description, files) {
  const text = stripComments(description || "");
  const sections = sectionsOf(text);
  const problems = [];
  for (const name of SECTIONS) {
    if (!sections.has(name)) problems.push(`The "${name}" heading is missing. Keep every heading of the template.`);
  }
  for (const name of SECTIONS.filter((n) => !["Platforms", "Checklist"].includes(n))) {
    if (sections.has(name) && !hasWords(sections.get(name))) problems.push(`"${name}" has no words of your own.`);
  }
  const rust = files.some((f) => f.startsWith("crates/") && f.endsWith(".rs"));
  const evidence = sections.get("Evidence") || "";
  if (sections.has("Evidence") && !hasVideo(evidence)) problems.push(`"Evidence" links no video. Record one with tools/record-evidence.ps1 and drag the .mp4 into the description.`);
  const perf = sections.get("Performance") || "";
  if (rust && sections.has("Performance") && !hasNumbers(perf)) problems.push(`"Performance" has no measured number with a unit (ms, fps, %, F1). This pull request changes Rust code in crates/.`);
  const tests = sections.get("Test coverage") || "";
  if (rust && sections.has("Test coverage") && !/\btests?\b|No new test:|\b[a-z0-9]+(_[a-z0-9]+){2,}\b/i.test(tests)) problems.push(`"Test coverage" names no test. A change to crates/ adds one, or says "No new test:" and why.`);
  const platforms = sections.get("Platforms") || "";
  if (sections.has("Platforms") && !/-\s*\[[xX]\]/.test(platforms)) problems.push(`Tick at least one platform under "Platforms".`);
  const checklist = sections.get("Checklist") || "";
  const open = (checklist.match(/-\s*\[ \]/g) || []).length;
  if (sections.has("Checklist") && open > 0) problems.push(`${open} checklist box${open === 1 ? " is" : "es are"} not ticked.`);
  return problems;
}

/**
 * Calls the GitHub REST API.
 * @param path - the API path
 * @param init - fetch options
 */
async function api(path, init = {}) {
  const res = await fetch(`https://api.github.com${path}`, { ...init, headers: { authorization: `Bearer ${process.env.GITHUB_TOKEN}`, accept: "application/vnd.github+json", "content-type": "application/json" } });
  if (!res.ok) throw new Error(`${init.method || "GET"} ${path}: ${res.status} ${await res.text()}`);
  return res.status === 204 ? null : res.json();
}

/** Reads the event, checks the description, and posts or updates the one comment. */
async function main() {
  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, "utf8"));
  const pr = event.pull_request;
  const repo = event.repository.full_name;
  const files = (await api(`/repos/${repo}/pulls/${pr.number}/files?per_page=100`)).map((f) => f.filename);
  const problems = checkDescription(pr.body, files);
  const comments = await api(`/repos/${repo}/issues/${pr.number}/comments?per_page=100`);
  const mine = comments.find((c) => c.body.includes(MARKER));
  const body = problems.length ? `${MARKER}\nThe description is not finished yet:\n\n${problems.map((p) => `- ${p}`).join("\n")}\n\nSee docs/pull-requests.md.` : `${MARKER}\nThe description has every section, a video and the numbers.`;
  if (mine) await api(`/repos/${repo}/issues/comments/${mine.id}`, { method: "PATCH", body: JSON.stringify({ body }) });
  else if (problems.length) await api(`/repos/${repo}/issues/${pr.number}/comments`, { method: "POST", body: JSON.stringify({ body }) });
  if (problems.length) {
    console.error(problems.join("\n"));
    process.exit(1);
  }
  console.log("The description is complete.");
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/").split("/").pop())) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
