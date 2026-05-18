# Migration & GitHub Setup — One-Time Instructions

> Delete this file after you've completed the steps below. It exists only as a guided runbook for the one-time move from the old nested location and the first push to GitHub.

## Status

- ✅ All files copied from `/Users/bkunyiha/Rust/FDAP/How_Query_Engines_Work/how-query-engines-work-rust/` to `/Users/bkunyiha/Rust/how-query-engines-work-rust/` (this directory)
- ✅ `ARCHITECTURE.md` created (sanitized technical reference for sharing)
- ✅ `README.md` rewritten to be standalone (no FDAP-folder cross-references)
- ✅ `TRANSLATION_NOTES.md` updated to reference local `ARCHITECTURE.md`
- ✅ All 77 Rust source files' doc-comment headers updated to reference local `ARCHITECTURE.md`
- ✅ All `Cargo.toml` files (workspace + 15 per-crate) updated; no stale FDAP-folder references
- ✅ Verified: zero references back to `/Users/bkunyiha/Rust/FDAP/`

## What you still need to do (in order)

### 1. Verify the new location builds cleanly

```bash
cd /Users/bkunyiha/Rust/how-query-engines-work-rust
cargo check --workspace      # downloads deps, type-checks all 15 crates
cargo build --workspace      # full compile of empty-bodied stubs
```

If both pass with zero warnings, the migration is good.

### 2. Delete the old location

The original directory at `/Users/bkunyiha/Rust/FDAP/How_Query_Engines_Work/how-query-engines-work-rust/` is now redundant. Delete it:

```bash
rm -rf /Users/bkunyiha/Rust/FDAP/How_Query_Engines_Work/how-query-engines-work-rust/
```

### 3. Update the FDAP planning docs that referenced the old location

Several FDAP-folder docs still point at the old absolute path. Fix them:

```bash
cd /Users/bkunyiha/Rust/FDAP
# Update Translation Plan and other FDAP-folder docs to reference the new location.
# (Cowork can do this for you in the next session — just say "update FDAP docs
# to reflect the new rquery location at /Users/bkunyiha/Rust/how-query-engines-work-rust/")
```

### 4. Initialize git in the new location

```bash
cd /Users/bkunyiha/Rust/how-query-engines-work-rust
git init -b main
git add .
git commit -m "Initial scaffold: 15-crate workspace, file-level 1:1 stubs, ARCHITECTURE.md"
```

### 5. Create the GitHub private repository

If you have the GitHub CLI (`gh`) installed and authenticated:

```bash
gh repo create bkunyiha/how-query-engines-work-rust \
  --private \
  --description "Faithful Rust port of Andy Grove's kquery (the Kotlin query engine from 'How Query Engines Work', 2nd edition)" \
  --source . \
  --remote origin \
  --push
```

If you don't have `gh`, do it through the web UI:
1. Go to <https://github.com/new>
2. Repository name: `how-query-engines-work-rust`
3. Description: `Faithful Rust port of Andy Grove's kquery (the Kotlin query engine from "How Query Engines Work", 2nd edition)`
4. Visibility: **Private** (this is critical until you decide to share publicly)
5. Do NOT initialise with README, .gitignore, or LICENSE (we already have them)
6. Create the repo, then:

```bash
git remote add origin git@github.com:bkunyiha/how-query-engines-work-rust.git
git push -u origin main
```

### 6. Verify the GitHub repo

Open <https://github.com/bkunyiha/how-query-engines-work-rust> in your browser. You should see:
- README.md rendered as the landing page
- ARCHITECTURE.md, TRANSLATION_NOTES.md, LICENSE visible at root
- 15 crate directories
- The "Private" badge next to the repo name

### 7. Update the Cargo.toml `repository` URL

The workspace `Cargo.toml` has:
```toml
repository = "https://github.com/bkunyiha/how-query-engines-work-rust"
```

This is already correct as long as the GitHub repo is at that exact path. If you used a different repo name, update this field accordingly.

### 8. (Later, for Phase 4) Add Andy Grove as a collaborator

This step happens *after* the Phase 3 book rewrite is complete and you're ready to send the outreach email. Until then, the repo stays private to you only.

When Phase 4 arrives:
- GitHub UI: Repository → Settings → Collaborators → Add `andygrove`
- Permission level: **Read** (or **Triage** if you want him to be able to open issues / comment on PRs)
- He'll receive an email invitation; once he accepts, he can clone and read the repo

Or via `gh`:
```bash
gh api repos/bkunyiha/how-query-engines-work-rust/collaborators/andygrove \
  -X PUT -f permission=pull
```

### 9. Delete this file

Once you've completed the setup:

```bash
rm /Users/bkunyiha/Rust/how-query-engines-work-rust/MIGRATION_INSTRUCTIONS.md
git add -A && git commit -m "Remove one-time migration instructions"
```

## Recap of the share-safe surface

What Andy will see when he clones the private repo:

- **`README.md`** — what the project is, how it relates to his book, build instructions, key translation conventions, link to ARCHITECTURE.md for depth
- **`ARCHITECTURE.md`** — porting methodology, idiom cheatsheet, module-by-module plan, empirical-findings table (the rigor demonstration), the KQuery→RQuery rebrand rule, the visitor-doesn't-apply analysis, the coroutines→Rayon substitution
- **`TRANSLATION_NOTES.md`** — audit log of deliberate divergences (currently empty per-module sections; populated during the port)
- **`LICENSE`** — Apache-2.0, matching his upstream `kquery`
- **`Cargo.toml`** + 15 crates + `proto/` + `testdata/`

What Andy will NOT see:

- Your FDAP folder with the strategic planning docs
- Your personal thesis or career-strategy material
- The marketing playbook
- The outreach email template for him
- Anything mentioning Phase 4 or Phase 5 or the broader career transformation
