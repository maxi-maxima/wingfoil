# The final cutover — runbook

This is the sequence that removes the legacy tree and the scaffolding that
existed only to let the two engines coexist.

> ⛔ **Do not start yet.** This file used to open "everything else is done". It
> is not: the pre-flight sweep below was run on 2026-08-07 and **came back
> non-empty**, reopening `cutover-plan.md` §3 as rows **3.10–3.12**. Those are
> unruled parity items, and the whole point of the pre-flight is that they get
> dealt with *before* the deletion, not after. Steps 1–8 are correct and ready;
> the gate in front of them is not green.

[`cutover-plan.md`](cutover-plan.md) holds the *why* and the audit trail of
rulings; this file is the *how*, written to be executed. Figures are counted
from `next` @ `f769d56` — re-count before starting, since they drift with any
merge.

## What is and is not reversible

Worth being precise, because "irreversible" is doing a lot of work in
conversation about this step.

**Recoverable.** The legacy source is in git history forever, and
`wingfoil` 8.0.0 / `wingfoil-derive` 8.0.0 / `wingfoil-python` 8.0.0 /
`wingfoil-wire-types` 8.0.0 stay on crates.io permanently — crates.io does not
delete. Existing lockfiles keep resolving. Nothing a downstream user has today
stops working.

**Not recoverable.** The *comparison* between the engines. Once legacy is gone
you cannot run a legacy-vs-wingfoil benchmark or a cross-engine wire test again
without reviving the tree. That is why **gate 6.4 was read and its numbers
written into `cutover-plan.md` before this step** — that capture is the
permanent record, and re-running it later is not an option.

Everything else here is mechanical and re-doable.

## Pre-flight

Run these before touching anything. Stop if any fails.

```bash
# The sandbox clone is shallow — without this the window silently truncates and
# the sweep returns a falsely clean result.
git fetch --unshallow

# 6.5 — legacy drift sweep. Anything legacy-originated since the 3.9 sweep is a
# parity target that has to be dealt with BEFORE deletion, not after.
git log --format='%h %ad %s' --date=short 754514c..HEAD -- legacy/

# CI green on wingfoil, and the working tree clean.
git status --porcelain
```

The sweep is expected to return only cutover-mechanics commits (the workspace
split, the rename alias, `legacy/.gitignore`). Anything else — a real change to
legacy source — means someone was still working in that tree.

**On the 2026-08-07 run, something else was there**: `f5f22d5` (#667), legacy's
fluent `GraphBuilder`, which landed on `main` the day after the 3.9 sweep
declared the tree "frozen in practice". It is now `cutover-plan.md` row 3.10.
Reading the two public surfaces against each other at the same time — which the
commit log cannot do, because an absence in wingfoil is not a commit in legacy —
added rows 3.11 (`Graph::print()`) and 3.12 (`demux_it_with_map`).

Two things follow for whoever runs this next:

1. **Run both instruments, not just the sweep.** The log tells you what legacy
   gained; only a surface diff tells you what wingfoil never had. 3.9's
   structural cross-check was at *file* granularity, which is too coarse to see
   a missing trait method — both 3.11 and 3.12 live in files that pass it.
2. **A clean sweep is a claim about a snapshot, not an invariant.** Re-run it
   on the deletion branch itself, immediately before merging, however recently
   it last came back empty.

Land the deletion as **one PR with the tree quiet**, for the same reason the
rename was: it touches everything, so anything open across it conflicts.

---

## Step 1 — delete the tree

```bash
git rm -r legacy/
```

That is cutover-plan **1.3** (the legacy `wingfoil-derive` crate) and the
deletion half of **4.3** (the `legacy/` copies of README / CONTRIBUTING /
CLAUDE.md) in one move. Nothing under `crates/` depends on the legacy crates —
that invariant has been enforced since the dependency inversion, and it is what
makes this a deletion rather than an unpick.

## Step 2 — remove the `legacy_wingfoil` alias

The alias existed because a package cannot depend on another of its own name.
With legacy gone there is nothing to alias.

**`crates/wingfoil/Cargo.toml`** — three entries:

| line | what | action |
|---|---|---|
| dev-dep | `legacy_wingfoil = { package = "wingfoil", path = "../../legacy/wingfoil" }` | delete |
| `iceoryx2` feature | `"legacy_wingfoil/iceoryx2"` | drop from the list |
| `zmq-cross-engine-test` feature | `"zmq", "legacy_wingfoil/zmq"` | delete the whole feature |

**Three files use it, and all three are deletions, not rewrites** — each exists
to compare against an engine that no longer exists:

- `crates/wingfoil/tests/engine_semantics.rs` — the parity oracle.
- `crates/wingfoil/tests/zmq_cross_engine_integration.rs` — proved the two
  engines agree on the wire. Its sibling
  `zmq_cross_lang_integration.rs` **stays**: that one tests Rust ↔ Python,
  which survives the cutover.
- `crates/wingfoil/benches/tiers.rs` — the `legacy` arm of each group only.
  **The bench itself stays**; strip the legacy bars and their imports, keeping
  interpreted / compiled / nested. Also drop its `[[bench]]`-adjacent legacy
  references and the `legacy` group labels.

Do not delete `tiers.rs` wholesale — the three surviving tiers are still the
tier-comparison benchmark.

## Step 3 — revert the package-selection workaround

`-p wingfoil` was ambiguous only because two packages carried that name. Verify
it is no longer, then revert:

```bash
cargo check -p wingfoil --lib      # must now resolve, not error
```

- **20 workflow lines** and **73 docs/skills files**:
  `--manifest-path crates/wingfoil/Cargo.toml` → `-p wingfoil`.
- **28 lines** referencing `--manifest-path legacy/...` — these go with the
  workflows that own them (step 4) or are docs that die with the tree.
- Restore `-p wingfoil-python` where it was rewritten to a manifest path.

Take the same care the rename needed: the pattern is `-p wingfoil(?![-\w])`.
A bare `-p wingfoil\b` **also matches `-p wingfoil-python`**, because a hyphen
is a word boundary — that mistake cost a CI round during 1.2.

## Step 4 — retire the legacy workflow set (5.2)

The collapse already happened, ahead of this runbook: the wingfoil workflows
own the plain filenames and every legacy twin carries a `legacy-` prefix. All
that is left here is deletion.

Delete these thirteen:

`legacy-adapter-integration.yml`, `legacy-aeron-integration.yml`,
`legacy-augurs-integration.yml`, `legacy-etcd-integration.yml`,
`legacy-iceoryx2-integration.yml`, `legacy-kafka-python-integration.yml`,
`legacy-kdb-integration.yml`, `legacy-otlp-integration.yml`,
`legacy-postgres-integration.yml`, `legacy-prometheus-integration.yml`,
`legacy-python-test.yml`, `legacy-redis-integration.yml`,
`legacy-zmq-etcd-integration.yml`.

Then drop their `legacy-*` job entries from `integration-tests.yml`, drop the
`legacy-python-test` job from `all-tests.yml`, and drop the `test-legacy` and
`lint-legacy` jobs from `rust-test.yml`.

**`legacy-augurs-integration.yml` has no wingfoil twin by design** — wingfoil's
augurs tests run inside `rust-test.yml` under `--all-features`. Retire it; there
is nothing to fold into.

**Repoint the latency-e2e workflows — ✅ done, and this row's premise was
wrong.** It said the workflows still built from `legacy/`. They did not: the
tree inversion (`754514c`, #655) had already moved them onto the wingfoil-side
copy. What it missed is that the very next commit — `e68e1c6` (#656), which
regrouped examples into `core/` / `adapters/` / `showcase/` — moved the example
to `showcase/latency_e2e/` and did **not** update the workflows. The crate
rename (`36596a2`) then carried the broken path forward mechanically.

So `build-latency-e2e-images.yml`, `build-latency-e2e-ami.yml` and
`deploy-latency-e2e.yml` have been pointing at `crates/wingfoil/examples/
latency_e2e/` — a path that has not existed since #656 — and `bump.yml`'s
importmap pin was wrong the same way. All four now name
`crates/wingfoil/examples/showcase/latency_e2e/`, verified to resolve on disk,
and `js/README.md`'s pointer at `static/app.js` with them. Nothing surfaced the
breakage because all three latency-e2e workflows are `workflow_dispatch`-only.

What is still owed here at deletion: `bump.yml` rewrites the `@wingfoil/client`
importmap pin in **both** trees' `static/index.html`; drop the
`legacy/wingfoil/examples/latency_e2e/static/index.html` arm from that loop and
the `for` loop with it.

> The general lesson, worth applying to the rest of this step: a path repoint
> that is never exercised is not verified by CI. Check each rewritten path
> resolves on disk before merging.

> ⚠️ **Check names change.** Deleting a workflow removes its CI check. If the
> repository has required status checks configured on `main` or `next`, they
> must be updated in the same window or merges will block on checks that can
> never report. This is the one step with a consequence outside the repo.

## Step 5 — docs

- Root `CLAUDE.md`: remove the legacy branching section and the
  "Working under `legacy/`?" banner. The two-branch workflow (`main` for
  legacy, `next` for everything else) ends here — there is only one tree.
- `docs/migration.md`: keep it. It is for users migrating *off* the legacy
  engine, and is more useful after the deletion, not less.
- `crates/README.md` and the architecture doc: drop the `legacy/` row and any
  "parity oracle" framing that is now historical.
- `docs/port-plan.md` / `cutover-plan.md`: mark Phase 7 complete. Keep the
  rulings and the gate 6.4 numbers — that is the audit trail.

## Step 6 — gates on the promoted tree

```bash
cargo fmt --all -- --check          # 6.1
cargo lint
cargo lint-all                      # needs aeron's toolchain: cmake >= 3.30
cargo test -p wingfoil --all-features   # 6.2
cd crates/wingfoil-python && maturin develop && pytest
```

Read exit codes directly — piping into `head`/`tail` masks them.

**6.3**: every integration workflow green on the cutover branch. They gate the
service-backed adapters the unit suites cannot reach.

**6.4** is already banked in `cutover-plan.md` and cannot be re-run. Do not
treat its absence from this list as an oversight.

## Step 7 — the swap itself ✅ spent

This step assumed the work was sitting on `next` with `main` still carrying the
pre-cutover world. That is no longer true, and all three items are now closed:

1. ~~Open the `next` → `main` PR.~~ ✅ Merged at `af73401`. `main` is the trunk
   for both trees, and the `next` branch has since been **deleted** — four
   branches remain on the remote and it is not among them.
2. ~~Update the `[main, next]` branch filters.~~ ✅ `CLAUDE.md`'s condition was
   "strip them when the branch is actually deleted, not before"; it is, so they
   are. `rust-test.yml`, `python-test.yml` and `security-audit.yml` are back to
   `branches: [ "main" ]`, with `rust-test.yml`'s four cache `save-if` guards
   and the `main/next` concurrency comments. Two corrections to the list this
   step carried: `all-tests.yml` and `rust-fmt.yml` never had a `next` filter,
   and `python-test.yml` / `security-audit.yml` — unlisted here — did.
   **The `legacy-*` workflows still say `main/next` in their comments, on
   purpose**: they are deleted wholesale at step 4, so editing them is churn.
3. ~~`CONTRIBUTING.md`'s branching table.~~ ✅ Replaced with the single-trunk
   rule, matching `CLAUDE.md`.

## Step 8 — issues

**The re-labelling is already done** — all 26 open issues carry `next`, and
nothing is left under `classic`. What this step still owes:

- **Close #367** (iceoryx2/aeron missing from the wheel) — resolved by the
  5.4 wheel change.
- Re-check the survivors against the deleted tree: **#450** wheels, **#452**
  dependabot, **#449 / #451 / #359** CI, **#461** supply chain, **#457**
  wingfoil-js, **#437** web historical streaming. All describe the surviving
  engine or its packaging, so they stay open — but **#437** in particular
  should be confirmed against wingfoil's web adapter rather than assumed to carry
  over, and the CI issues (#449 / #451) are partly answered by step 4's
  workflow collapse.
- Anything that still describes the *deleted* engine can be closed with a note
  pointing at this runbook.

## Order

Steps 1–4 are one PR: each breaks the tree on its own, and only the combination
compiles. 5 can ride along. 6 gates it. 7 is a separate PR by nature. 8 is
independent and can happen any time.
