# Chapter IV: The Workflow

*Or: How code travels from a laptop in a coffee shop to a container on a server.*

---

It is a Thursday afternoon. Alice is not at the office. She's at a coffee shop with her laptop, working on the authentication module for `backend-api`.

She has the MELISA client installed. She registered the server when she onboarded:

```bash
melisa auth add saferoom root@192.168.1.100
```

Her connection profile is stored. SSH multiplexing is configured. When she types a MELISA command, it's nearly instant — no TCP handshake, no key exchange, the master connection has been alive for hours.

She clones the project to her local machine:

```bash
melisa clone backend-api
```

```
[INFO] Establishing secure channel to root@192.168.1.100...
[INFO] Cloning project 'backend-api' from remote master...
Cloning into './backend-api'...
[SUCCESS] Project 'backend-api' cloned successfully.
[INFO] Project registered in local registry.
```

The registry entry is created: `backend-api|/home/alice/projects/backend-api` in `~/.config/melisa/registry`. From this point on, any `melisa sync` command run from inside that directory tree knows exactly which project to push to.

---

## The Work

Alice edits files. She tests locally. She edits more files. This is normal programming. Nothing about MELISA changes the actual act of writing code.

What MELISA changes is what happens when she's done.

She runs:

```bash
cd backend-api
melisa sync
```

And this is what happens, in order:

**Step 1 — Context identification.** The `sync` command reads `~/.config/melisa/registry` and finds the longest matching parent path for the current directory. It identifies this as project `backend-api`.

**Step 2 — Stage everything.**
```bash
git add .
```

**Step 3 — Auto-commit.**
```bash
git commit -m "melisa-sync: 2026-03-20 16:30" --allow-empty
```
The `--allow-empty` flag is non-negotiable: if Alice runs `sync` twice without making changes, the second commit needs to succeed anyway so the server gets the `--update` trigger.

**Step 4 — Force push.**
```bash
git push -f origin master
```
Force push because sync is opinionated: local state wins. If there's a conflict between what's on the server and what's on Alice's laptop, Alice's laptop wins. This is a deliberate choice for a tool meant to push work to a server, not merge collaborative branches.

**Step 5 — Server-side update.** Over the existing multiplexed SSH connection:
```bash
ssh root@192.168.1.100 "melisa --update backend-api --force"
```
The server performs a hard reset of Alice's working copy to the just-pushed state.

**Step 6 — .env file sync.**
```bash
rsync -azR .env ./config/.env root@192.168.1.100:~/backend-api/
```
The `.env` files are `.gitignore`d — they contain secrets. They travel via rsync instead. The `-R` flag preserves relative paths: `./config/.env` lands at `~/backend-api/config/.env` on the server, not at `~/backend-api/.env`.

Total time for all six steps: under three seconds over the multiplexed connection.

---

## Running the Code

Alice wants to test her changes in the actual container environment, not just locally. She runs her test suite inside `alice-dev`:

```bash
melisa run alice-dev run_tests.sh
```

The shell script is streamed — not uploaded, not executed in a subshell, but piped directly through SSH:

```bash
cat run_tests.sh | ssh root@192.168.1.100 "melisa --send alice-dev bash -"
```

Output flows back in real time. Test results appear line by line. She watches the green dots.

One test is interactive — a setup wizard that asks questions. She uses a different command:

```bash
melisa run-tty alice-dev setup_wizard.py
```

This one is more careful. The script is first compressed and uploaded to `/tmp/` inside the container, then `ssh -t` allocates a full TTY for interactive execution. Her terminal is fully connected — stdin, stdout, stderr, and terminal size all pass through. She can type inputs, see progress bars, and interact normally. When it finishes, the script is automatically cleaned up from `/tmp/`.

---

## Bob's Perspective

Bob pushes his frontend changes at the same time. His workflow is identical to Alice's: `cd frontend-app && melisa sync`.

The `post-receive` hook fires on Bob's push. Alice's working copy is updated automatically. She didn't pull. She didn't have to.

When there's a conflict — Bob refactored an API endpoint Alice was consuming — they handle it in the usual Git way, locally. MELISA doesn't change how Git handles branches and merges. It just removes the friction of getting code onto the server.

---

## Erick's Perspective

Erick sees all of this from the server side. He can check the project state at any time:

```
melisa@saferoom-01:~> melisa --projects
```

```
[INFO] Scanning workspace for active projects...
  - backend-api   (/home/erick/backend-api)
```

If he wants to pull Alice's latest work into the master repository without waiting for her to push:

```
melisa@saferoom-01:~> melisa --pull alice backend-api
```

If he wants to reset everyone to the current master state after he commits a breaking change fix:

```
melisa@saferoom-01:~> melisa --update-all backend-api
```

If he needs to deploy the build artifact — a compiled binary, a `node_modules` directory, a directory of static assets — directly into a container:

```
melisa@saferoom-01:~> melisa --upload alice-dev ./dist/ /opt/backend-api/
```

The directory is compressed locally and streamed over SSH into the container. Nothing is buffered in memory. Large deployments move at network speed.

---

## End of Day

Alice finishes her session at the coffee shop. She closes her laptop. The SSH multiplexing socket times out after ten minutes. Everything she pushed is on the server. Her container is still running. Her working copy on the server has her latest code.

Tomorrow she'll open her laptop at home, run `melisa --list` to confirm the container state, and pick up exactly where she left off.

The server doesn't care where she is. It doesn't care what network she's on. As long as she can reach port 22, she has her full development environment.

---

## What Just Happened (The Dry Version)

The MELISA workflow has three tiers:

**Tier 1 — Local work.** Normal development. Any editor, any tools. Nothing special.

**Tier 2 — Sync.** One command. Six operations in sequence: stage, commit, push, server-update, env-sync. Two seconds. Code is on the server.

**Tier 3 — Remote execution.** Run code inside the container directly, with or without a TTY. Upload built artifacts. Forward arbitrary MELISA commands transparently.

The client is a thin orchestration layer. It doesn't store state — it reads SSH profiles from `~/.config/melisa/` and acts on them. It doesn't require the server to be configured for it specifically — it speaks to the server via SSH the same way any operator would.

The sum of these parts: a development environment that lives on a server, is instantly accessible from any machine, and propagates code changes automatically between team members.

Erick built it to end the Friday night "I think I broke something" messages. By most measures, it has.

---

*This concludes The MELISA Chronicles. For reference documentation, see [Part I: Getting Started](../getting-started/README.md) and the [Server CLI Reference](../server-cli/README.md).*