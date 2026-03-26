# Project Management Commands

## `--new_project <project_name>`

**Access:** Administrator only

Initializes a new master bare Git repository for collaborative use.

```
melisa@host:~> melisa --new_project myapp
```

**What happens:**

```
[SUCCESS] Master Git repository 'myapp' initialized and security protocols applied.
```

Internal steps:
1. Creates `/opt/melisa/projects/myapp/`
2. `git init --bare --shared=group /opt/melisa/projects/myapp/`
3. `git config --system --add safe.directory /opt/melisa/projects/myapp` (prevents "dubious ownership" errors)
4. `chown -R root:melisa /opt/melisa/projects/myapp/`
5. `chmod -R 2775 /opt/melisa/projects/myapp/` (SetGID bit ensures new files inherit the `melisa` group)
6. `git -C /opt/melisa/projects/myapp/ config core.sharedRepository group`
7. Writes the post-receive hook:
   ```bash
   #!/bin/bash
   sudo melisa --update-all myapp
   ```
8. `chmod +x /opt/melisa/projects/myapp/hooks/post-receive`

---

## `--invite <project_name> <user1> <user2> ...`

**Access:** Administrator only

Grants project access to one or more users by cloning the master repository into each user's home directory.

```
melisa@host:~> melisa --invite myapp alice bob carol
```

**Validation:** The master project must exist at `/opt/melisa/projects/<project_name>/`. If it doesn't, the command fails with a clear error.

**Per-user steps:**
1. Removes any existing (potentially corrupted) copy: `rm -rf /home/<user>/myapp`
2. Registers the master path as safe for this user's git operations
3. Runs `git clone /opt/melisa/projects/myapp /home/<user>/myapp` as that user's context
4. Sets group ownership and permissions on the working clone
5. Configures the remote to point to the master repository

Alice can now push to the project with `git push origin master` from `/home/alice/myapp/`.

---

## `--out <project_name> <user1> <user2> ...`

**Access:** Administrator only

Revokes project access from one or more users by removing their working clone.

```
melisa@host:~> melisa --out myapp alice
```

**Validation:** The master project must exist. The command fails if the project doesn't exist.

For each specified user, removes `/home/<user>/<project_name>/`. The master repository and other members' clones are unaffected. The user's commits that were already pushed to master remain in Git history.

---

## `--pull <from_user> <project_name>`

**Access:** Administrator only

Merges code from a specific user's working directory into the master repository.

```
melisa@host:~> melisa --pull alice myapp
```

This is a **force operation** — it copies Alice's working directory state into the master without requiring a standard `git push` flow. Useful for:
- Code review by admins who want to inspect and integrate a user's work
- Rescuing work from a user who can't push due to merge conflicts
- Emergency code retrieval from departing team members

---

## `--projects`

**Access:** All users

Lists all projects associated with the current user's workspace:

```
melisa@host:~> melisa --projects
```

Scans the user's home directory for directories that correspond to master projects in `/opt/melisa/projects/`.

---

## `--update <project_name> [--force]`

**Access:** All users (own workspace) · Admin (any user's workspace)

Synchronizes a user's working copy with the current state of the master repository.

```
melisa@host:~> melisa --update myapp
melisa@host:~> melisa --update myapp --force
melisa@host:~> melisa --update alice myapp --force   # Admin targeting Alice
```

**Argument parsing:**
- `melisa --update myapp` → updates the current user's copy of `myapp`
- `melisa --update alice myapp` → admin targeting Alice's copy of `myapp`
- `--force` flag can appear anywhere in the argument list and is extracted independently

**Without `--force`:** Performs a `git pull` or `git fetch + reset` that respects any local uncommitted changes.

**With `--force`:** Performs a hard reset to the master state, **discarding any local uncommitted changes** in the working directory.

> The `--force` flag is used by `exec_sync` on the client side to guarantee the server reflects the latest pushed state.

---

## `--update-all <project_name>`

**Access:** Administrator only

Propagates the current master repository state to **all invited members' working directories** simultaneously.

```
melisa@host:~> melisa --update-all myapp
```

This command is automatically triggered by the `post-receive` hook whenever any member pushes to the master:

```bash
# /opt/melisa/projects/myapp/hooks/post-receive
#!/bin/bash
sudo melisa --update-all myapp
```

The result: as soon as anyone pushes code, every team member's working copy on the server is instantly updated.

---

## `--delete_project <project_name>`

**Access:** Administrator only

Permanently destroys a project: the master repository and every member's working clone.

```
melisa@host:~> melisa --delete_project myapp
```

**Validation:** The master project must exist.

**What gets deleted:**
1. `/opt/melisa/projects/myapp/` — the entire master bare repository with all commit history
2. `/home/*/myapp/` — every member's working clone across all user home directories

> **This is irreversible. All Git history is permanently lost.**

---

## Project Workflow Cheatsheet

```bash
# Admin: set up a project
melisa --new_project myapp
melisa --invite myapp alice bob

# Users: work on the project
# (inside their MELISA shell or via client)
cd ~/myapp
echo "hello" > main.py
git add . && git push origin master
# → post-receive hook auto-runs --update-all

# Admin: bring a user's code into master
melisa --pull alice myapp

# Admin: force-sync all members
melisa --update-all myapp

# Admin: remove a member
melisa --out myapp alice

# Admin: tear down the project
melisa --delete_project myapp
```