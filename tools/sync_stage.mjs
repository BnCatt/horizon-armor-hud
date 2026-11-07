// Copies the files that differ from a fresh `horizon-mixins apply` output
// into the kept stage (other files keep their mtimes, so cargo rebuilds little).
// Usage: node tools/sync_stage.mjs <fresh stage> <kept stage>
import { readdirSync, readFileSync, writeFileSync, mkdirSync, statSync } from "node:fs";
import { join, dirname, relative } from "node:path";

const [fresh, kept] = process.argv.slice(2);
const skip = new Set([".git", "target"]);
let changed = 0;
function walk(dir) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (skip.has(e.name)) continue;
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else {
      const rel = relative(fresh, p);
      const dst = join(kept, rel);
      const a = readFileSync(p);
      let same = false;
      try { same = statSync(dst).size === a.length && readFileSync(dst).equals(a); } catch {}
      if (!same) {
        mkdirSync(dirname(dst), { recursive: true });
        writeFileSync(dst, a);
        changed++;
        console.log("updated", rel);
      }
    }
  }
}
walk(fresh);
console.log(`${changed} file(s) updated`);
