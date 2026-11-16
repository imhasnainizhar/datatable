// Compile and run every Rust sample in content/**/*.mdx against the crate.
//
//   npm run check-samples
//
// ```rust         compiled and run (assertions must hold)
// ```rust-panic   compiled and run, must panic
// ```rust-nocheck / ```rust-sig   skipped
//
// Samples without `fn main` are wrapped in one; if they use `?`, the wrapper
// returns Result<(), Box<dyn Error>>. Lines starting with "# " are included.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const site = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const repo = path.dirname(site);
const crate = path.join(repo, "target", "doc-samples");

function* mdxFiles(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* mdxFiles(p);
    else if (entry.name.endsWith(".mdx")) yield p;
  }
}

const samples = [];
for (const file of mdxFiles(path.join(site, "content"))) {
  const text = fs.readFileSync(file, "utf8");
  const re = /^```(rust(?:-panic)?)\r?\n([\s\S]*?)^```/gm;
  let m;
  let n = 0;
  while ((m = re.exec(text))) {
    n += 1;
    const lines = m[2]
      .replace(/\r/g, "")
      .replace(/\n$/, "")
      .split("\n")
      .filter((l, i) => !(i === 0 && /^\/\/ file: /.test(l)))
      .map((l) => (l.startsWith("# ") ? l.slice(2) : l === "#" ? "" : l));
    samples.push({ where: `${path.relative(site, file)} #${n}`, panic: m[1] === "rust-panic", lines });
  }
}

const stripForScan = (s) => s.replace(/\/\/[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'/g, "");

const modules = [];
const tests = [];
samples.forEach((s, i) => {
  const uses = s.lines.filter((l) => l.startsWith("use "));
  let body = s.lines.filter((l) => !l.startsWith("use ")).join("\n");
  let returns;
  if (/\bfn main\s*\(/.test(body)) {
    returns = /\bfn main\s*\(\)\s*->/.test(body);
    body = body.replace(/\bfn main\s*\(/, "pub fn main(");
  } else {
    returns = stripForScan(body).includes("?");
    body = returns
      ? `pub fn main() -> ::std::result::Result<(), Box<dyn std::error::Error>> {\n${body}\nOk(())\n}`
      : `pub fn main() {\n${body}\n}`;
  }
  modules.push(`// ${s.where}\n#[allow(unused, clippy::all)]\nmod s${i} {\n${uses.join("\n")}\n${body}\n}\n`);
  tests.push(`#[test]${s.panic ? "\n#[should_panic]" : ""}\nfn s${i}() { s${i}::main()${returns ? ".unwrap()" : ""}; }\n`);
});

fs.mkdirSync(path.join(crate, "src"), { recursive: true });
fs.writeFileSync(
  path.join(crate, "Cargo.toml"),
  '[package]\nname = "doc-samples"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n\n[dependencies]\nflextable = { path = "../.." }\n\n[workspace]\n',
);
fs.writeFileSync(path.join(crate, "src", "lib.rs"), modules.join("\n") + "\n" + tests.join("\n"));

console.log(`Compiling and running ${samples.length} samples...`);
try {
  execFileSync("cargo", ["test", "--quiet", "--manifest-path", path.join(crate, "Cargo.toml")], {
    stdio: ["ignore", "pipe", "pipe"],
    encoding: "utf8",
  });
  console.log(`All ${samples.length} samples compiled and ran.`);
} catch (err) {
  let out = `${err.stdout ?? ""}${err.stderr ?? ""}`;
  samples.forEach((s, i) => {
    out = out.replace(new RegExp(`\\bs${i}\\b`, "g"), `s${i} [${s.where}]`);
  });
  console.error(out);
  process.exit(1);
}
