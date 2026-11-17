// Checks the Markdown pages against docs/writing-style.md: banned phrases, every page in docs/ listed in
// docs/README.md, and every relative link pointing at a file that exists. Exits 1 and lists each problem with its
// file and line.
//
//   node tools/doc-style/check.mjs            all pages
//   node tools/doc-style/check.mjs docs/x.md  one page

import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

/** The banned patterns, each with the advice printed when it matches. */
const RULES = [
  [/[—–]/, "an em or en dash: write two sentences"],
  [/\s-\s(?![-|])/, "a spaced hyphen used as a dash: write two sentences"],
  [/\b[A-Za-z]{2,}-[A-Za-z]{2,}\b/, "words joined by a hyphen: write them as separate words"],
  [/\b(worth noting|to be clear|honestly|simply put|put simply)\b/i, "an announcement: delete it"],
  [/\bthe whole point\b/i, "emphasis with no information: state the reason"],
  [/\b(load bearing|first class|battle tested|blazing fast)\b/i, "an invented technical word: say what it does"],
  [/\b(shape of|vocabulary)\b/i, "a vague noun: name the concrete thing"],
  [/\b(blows up|takes down)\b/i, "a dramatic failure verb: say what happens"],
  [/\brather than\b/i, "a contrast clause: say what is true and stop"],
  [/\b(colour|behaviour|centre|grey|analyse|optimise|licence)\b/i, "British spelling: use American spelling"],
];

/** The Markdown files to check. */
function pages(args) {
  if (args.length) return args.map((a) => resolve(a));
  const out = ["README.md", "CONTRIBUTING.md", "AGENTS.md", "SECURITY.md", "tests/fixtures/README.md"].map((p) => join(root, p));
  for (const f of readdirSync(join(root, "docs"))) if (f.endsWith(".md")) out.push(join(root, "docs", f));
  return out;
}

/**
 * Removes what the rules do not apply to: inline code, file paths, link targets and URLs. Code blocks and the
 * doc-style: off regions are skipped by the caller.
 * @param line - one line
 */
function prose(line) {
  return line.replace(/`[^`]*`/g, "").replace(/\S*[\/.]\S*-\S*|\S*-\S*[\/.]\S*/g, "").replace(/\]\([^)]*\)/g, "]").replace(/https?:\/\/\S+/g, "").replace(/<!--.*?-->/g, "");
}

/**
 * Checks one page and returns its problems.
 * @param file - the page
 */
function checkPage(file) {
  const problems = [];
  const lines = readFileSync(file, "utf8").split(/\r?\n/);
  let code = false;
  let off = false;
  lines.forEach((line, i) => {
    if (/^\s*```/.test(line)) code = !code;
    if (line.includes("doc-style: off")) off = true;
    if (line.includes("doc-style: on")) off = false;
    for (const m of line.matchAll(/\]\(([^)#\s]+)(#[^)]*)?\)/g)) {
      const target = m[1];
      if (!/^[a-z]+:/i.test(target) && !existsSync(resolve(dirname(file), target))) problems.push(`${rel(file)}:${i + 1}: broken link ${target}`);
    }
    if (code || off || /^\s*```/.test(line)) return;
    const text = prose(line);
    for (const [re, why] of RULES) if (re.test(text)) problems.push(`${rel(file)}:${i + 1}: ${why}: ${line.trim().slice(0, 100)}`);
  });
  return problems;
}

/** A path relative to the repository. */
function rel(file) {
  return relative(root, file).replace(/\\/g, "/");
}

/** Every page in docs/ must be linked from docs/README.md. */
function checkIndex() {
  const index = readFileSync(join(root, "docs/README.md"), "utf8");
  return readdirSync(join(root, "docs"))
    .filter((f) => f.endsWith(".md") && f !== "README.md" && statSync(join(root, "docs", f)).isFile())
    .filter((f) => !index.includes(`](${f})`))
    .map((f) => `docs/${f}: not listed in docs/README.md`);
}

const args = process.argv.slice(2);
const problems = pages(args).flatMap(checkPage).concat(args.length ? [] : checkIndex());
if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} problem${problems.length === 1 ? "" : "s"}. See docs/writing-style.md.`);
  process.exit(1);
}
console.log("Documentation follows docs/writing-style.md.");
