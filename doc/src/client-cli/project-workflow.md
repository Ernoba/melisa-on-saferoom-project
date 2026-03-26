# Project Workflow

The project workflow commands handle **bidirectional synchronization** between your local machine and the MELISA host, as well as between your host workspace and the master repository.

---

## `melisa clone <n> [--force]`

Downloads a project workspace from the MELISA host to your local machine.

```bash
melisa clone myapp
melisa clone myapp --force
```

### Default Mode (Git Clone)

Without `--force`, performs a standard `git clone` from the master repository:

```bash
git clone ssh://root@192.168.1.100/opt/melisa/projects/myapp ./myapp
```

- Creates a `./myapp/` directory locally
- The local repo is connected to the master — you can `git push`/`git pull` normally
- Registers the project path in `~/.config/melisa/registry`

**Anti-nesting detection:** If your current directory is already named `myapp`, the clone target becomes `.` (in-place) instead of `./myapp/`, preventing a nested `myapp/myapp/` structure.

**Non-empty directory guard:** If cloning into `.` and the directory isn't empty, the command refuses and suggests `--force`:

```
[ERROR] Directory is not empty. Use '--force' for Rsync overwrite or navigate to an empty directory.
```

### Force Mode (Rsync Overwrite)

With `--force`, bypasses Git entirely and uses Rsync to copy the server-side working copy directly:

```bash
rsync -avz --progress root@192.168.1.100:~/myapp/ ./myapp/
```

This is useful when:
- The master repo doesn't yet have any commits (empty repo)
- You want an exact copy of someone's working state, not just committed history
- Git clone is failing due to remote configuration issues

After completion, shows a workspace state summary:

```
[Workspace State: ./myapp]
 => Files: 42
 => Dirs:  8
 => Size:  1.2M

Project Topology (Depth 2):
  src/
  src/main.rs
  src/lib.rs
  Cargo.toml
  README.md
  ...
```

---

## `melisa sync`

Pushes your local changes to the server and triggers a server-side update.

```bash
cd ~/projects/myapp
# ... edit files ...
melisa sync
```

### The Full Sync Pipeline

```
:: Synchronizing myapp [Branch: main]

 [INFO] Staging and committing local changes...
        git add .
        git commit -m "melisa-sync: 2026-03-20 16:30"

 [INFO] Transmitting delta to host server...
        git push -f origin main

 [INFO] Triggering server-side update...
        ssh root@192.168.1.100 "melisa --update myapp --force"

 [INFO] Synchronizing environment configurations (.env)...
        rsync -azR ./.env root@192.168.1.100:~/myapp/

[SUCCESS] Host server is now perfectly synchronized with local state.
```

**Key behaviors:**

1. **Context detection:** `sync` identifies your project by scanning the path registry (`~/.config/melisa/registry`). If your current directory isn't registered as a MELISA project, it fails with a clear error. Register it by running `melisa clone` first.

2. **Auto-commit:** All changes are staged and committed automatically with a timestamp message. You don't need to run `git add` or `git commit` manually.

3. **Force push:** Uses `git push -f` to always succeed, even if there are diverged commits. This is intentional — `sync` is a "my local state is truth" operation.

4. **Post-push server update:** After pushing, `sync` SSHes into the server and runs `melisa --update myapp --force`, ensuring the server's working directory immediately reflects the pushed state.

5. **`.env` file handling:** Files matching `.env` within 2 directory levels are synced separately via Rsync using the `-R` (relative path) flag. This preserves the directory structure and handles the common case where `.env` files are in `.gitignore` but still needed on the server.

---

## `melisa get <n> [--force]`

Pulls the latest data from your **server-side working directory** to your local machine via Rsync.

```bash
melisa get myapp
melisa get myapp --force
melisa get              # auto-detects project from current directory
```

### Context Resolution

`get` uses a three-level fallback to find the project:

1. Explicit argument: `melisa get myapp`
2. Auto-detect from PWD via `db_identify_by_pwd()` (longest matching parent path)
3. If the current directory name matches a project name, use it

### Default Mode (Safe Sync)

```bash
rsync -avz --progress --exclude='.git/' --ignore-existing \
  root@192.168.1.100:~/myapp/ /local/path/myapp/
```

`--ignore-existing` means only files that don't exist locally are downloaded. Locally modified files are preserved. Good for "fill in what I'm missing" scenarios.

### Force Mode

```bash
rsync -avz --progress --exclude='.git/' \
  root@192.168.1.100:~/myapp/ /local/path/myapp/
```

Without `--ignore-existing`, all files are overwritten with server versions. Good for "give me exactly what the server has" scenarios.

**Note:** `.git/` is always excluded from the Rsync to prevent corrupting your local Git repository state.

---

## Workflow Patterns

### Solo Developer Pattern

```bash
# Initial setup
melisa clone myproject

# Daily work loop
cd myproject
# ... edit code ...
melisa sync                   # push local → server
melisa get                    # pull server output → local (build artifacts, etc.)
```

### Team Collaboration Pattern

```bash
# Admin sets up
melisa --new_project teamapi
melisa --invite teamapi alice bob

# Alice's workflow (via client)
melisa clone teamapi
cd teamapi
# ... implement feature ...
melisa sync                   # push: server auto-updates all members

# Bob's workflow (his server-side copy was auto-updated by post-receive hook)
melisa get teamapi             # pull latest to his local machine

# Admin code review
melisa --pull alice teamapi   # bring Alice's work into master
```

### `.env` Synchronization

Environment files that are deliberately excluded from Git are handled transparently:

```
myproject/
├── .gitignore   ← contains ".env"
├── .env         ← NOT in git, synced via rsync during `melisa sync`
├── src/
└── ...
```

After `melisa sync`, both the committed code and the `.env` file exist on the server's working directory.