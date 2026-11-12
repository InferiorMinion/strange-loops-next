# Lean mutation runner

Run `node model/mutate/run.mjs --list` to inspect the catalogue.
Listing validates every replacement against the current source without changing files.
The runner tests check all 236 entries and their unique source sites.
The catalogue retains 187 original entries, retires two mutations, and adds eight guard mutations.
Six more entries cover the D1 base boundary and the D2 code-unit reader.
Nine more cover D8's `of`: the thing, the two envelope checks, `of-not-root`, the batch thing check, labels in `of`, `from.of` and `env.of`.
Ten more cover issue #1 and labels in `under`: the shape stage's `envelope-not-object`, its order and a draft without `env`, the reader's unreadable field, set and kind, the salt reader, and labels in `under`.
Nine more cover `follow` on count, sum, min and max: the pool, two slots that drop it, a count that chooses before working out heads or lets an unknown payload test outrank a failing envelope test, the parser and the parsed form; two older entries are retargeted at the new grammar.
Seven more cover `by`: grouping before admission or before the heads, empty groups, an unknown `where` counted in a group, one membership for a record with several keys, an unreadable payload key counted as ungrouped, and `by` on collect with `follow`.
The source paths may name the Loops or Gen root; flat filenames still name Loops sources.
Run `node model/mutate/run.mjs --only things-prev-or,things-drop-may` to select mutations.
Use `--output path` to choose the JSON result path.
Use `--timeout seconds` to set each command timeout.
The default timeout is 180 seconds.
The default result path is `model/mutate/results.json`.
Run `node --test model/mutate/run.test.mjs` to check the runner.
See [REPORT.md](REPORT.md) for audited outcomes and the known generator assertion.

The runner requires clean candidate source files in the index and worktree.
It checks the baseline build and compares freshly generated vectors with `vectors/`.
It stops if the baseline differs.
Each mutation uses a separate generation directory with a copy of `vectors/`.
Gen replaces its generated directories and leaves the standard fixtures and README files unchanged.
It restores the source after each mutation and rebuilds the baseline at the end.
The runner uses a lock directory at `model/.mutation-run.lock`.
If a crash leaves the lock, inspect the source and processes before removing it.

Results are saved after each mutation.
Command logs are in the output path plus `.logs`.
A nonzero build is `rejected-by-build`, with `buildRejectionNeedsAudit` set.
Inspect its log to locate the rejection; the status makes no semantic-proof claim.
A timeout, spawn failure, or generator failure is an error, not a kill.
Survivors do not cause a nonzero exit.
Signals stop the active process group and return a signal-specific exit code.
This runner needs POSIX process groups, Git, Node, and Lake.
