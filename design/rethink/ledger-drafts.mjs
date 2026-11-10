// The decision ledger as drafts, one per line, for `loops emit -`:
//
//   node design/rethink/ledger-drafts.mjs | loops emit -
//
// Each decision is about nothing, so it anchors its own thing: revising one later is
// `loops emit decision --prev <id> ...`, and `loops about <id>` shows where it stands.
// The page still builds from ledger.json; this is the way out of it, not yet the way in.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ledger = JSON.parse(readFileSync(join(here, "ledger.json"), "utf8"));
for (const { kind, ...payload } of ledger) {
  process.stdout.write(JSON.stringify({ kind, payload }) + "\n");
}
