# Security Model

MELISA employs a multi-layered security architecture. Understanding these layers helps you make informed decisions about deployment and user trust.

---

## Layer 1: Physical Handshake (Setup)

The `--setup` routine contains an explicit SSH detection check. If the calling process is connected via SSH (detected by checking the `SSH_CLIENT` and `SSH_TTY` environment variables), setup aborts immediately:

```
[SECURITY] Remote session detected. --setup requires a physical terminal.
```

**Why?** An attacker who compromises your network before the host is fully configured should not be able to remotely bootstrap the MELISA environment. The initial `--setup` must happen with you physically present at the machine.

---

## Layer 2: The Jail Shell

MELISA's binary is registered in `/etc/shells` and assigned as the **login shell** for all MELISA users:

```
/etc/passwd:
alice:x:1001:1001::/home/alice:/usr/local/bin/melisa
```

When Alice SSHes in, she gets the MELISA prompt — not bash. She can only run MELISA commands. The `execute_command` function in `executor.rs` is the gatekeeper that parses and validates every command before execution.

The **bash passthrough** (the `_` arm in the command router) does allow arbitrary shell commands — this is intentional, as standard users need to be able to run `git`, `ls`, `cargo build`, etc. inside the MELISA session. However, this passthrough runs with the user's own privileges, not escalated ones.

---

## Layer 3: SUID Binary with Targeted Escalation

The MELISA binary at `/usr/local/bin/melisa` has the **SUID bit** set (`chmod 4755`). This means it runs as root regardless of who invokes it, but only the code inside `melisa` executes with those privileges.

When `main.rs` starts:
1. It checks if the current effective UID is root (`check_root()`)
2. If not, it re-launches itself via `sudo -H /usr/local/bin/melisa` with the original arguments
3. The sudoers rule `ALL ALL=(ALL) NOPASSWD: /usr/local/bin/melisa` makes this escalation passwordless

**Why not just set SUID and not use sudoers?** SUID alone doesn't propagate to child processes (like `lxc-create`, `useradd`, etc.). The sudoers rule ensures that when MELISA spawns sub-processes via `sudo <command>`, those calls also succeed without password prompts.

---

## Layer 4: Surgical Sudoers Policies

Rather than giving users full `sudo` access, MELISA generates per-user sudoers files that whitelist **exactly the binaries MELISA needs to function**, with glob patterns for arguments:

```
# /etc/sudoers.d/melisa_alice
alice ALL=(ALL) NOPASSWD: \
  /usr/bin/lxc-*, \
  /usr/share/lxc/templates/lxc-download *, \
  /usr/bin/git *, \
  /usr/local/bin/melisa *, \
  /usr/bin/mkdir -p *, \
  /usr/bin/rm -f *, \
  /usr/bin/bash -c *, \
  /usr/bin/tee *, \
  /usr/bin/chattr *
```

Alice cannot run `sudo rm -rf /` because `rm -rf /` doesn't match the whitelisted pattern `rm -f *` (which allows `rm -f <specific-file>`).

---

## Layer 5: Home Directory Isolation

Two `chmod` operations create a privacy boundary:

```bash
chmod 711 /home         # set during --setup
chmod 700 /home/alice   # set when alice is created
```

- `711` on `/home`: Everyone can traverse (`x` bit) into subdirectories, but the **list** bit is not set — so `ls /home` shows nothing. Users cannot enumerate who else is on the system.
- `700` on `/home/alice`: Only Alice and root can read, write, or list her home directory. No peeking.

---

## Layer 6: Namespace Isolation (LXC)

LXC containers use Linux kernel namespaces to provide isolation:

| Namespace | What it isolates |
|-----------|-----------------|
| `pid` | Process IDs — container processes can't see host processes |
| `net` | Network — container has its own interfaces, routing tables |
| `mnt` | Mount points — container has its own filesystem hierarchy |
| `uts` | Hostname — container can have a different hostname |
| `ipc` | IPC — message queues, semaphores are isolated |
| `user` | UID/GID — container root (UID 0) maps to unprivileged UID 100000 on the host |

The user namespace mapping (`subuid`/`subgid` range `100000–165535`) means that even if a process inside the container gets "root" inside the container, it's actually UID 100000 on the host — an unprivileged user with no special permissions.

---

## Layer 7: Command History Security

The MELISA shell maintains a command history file. When history is cleared via `melisa --clear`:

1. The in-memory buffer is flushed
2. The history file is deleted (TOCTOU-safe: delete first, don't check existence first)
3. A new empty file is written
4. **Strict permissions (`0600`) are applied** — only the owner can read or write the history file

This prevents other users (and even root processes running as other users) from reading sensitive command history that might contain container names, project names, or file paths.

---

## Metadata Injection Security

The `inject_distro_metadata` function in `metadata.rs` implements two security checks:

1. **Path traversal prevention**: Container names containing `/`, `\`, or `..` are rejected with a `SecurityViolation` error before any filesystem operations occur.

2. **Atomic write pattern**: Metadata is written to a `.tmp` file first, permissions are set, then the file is renamed to the final name. This prevents readers from seeing a partially-written file.

---

## Security Considerations & Known Trade-offs

| Trade-off | Reason |
|-----------|--------|
| `sudo melisa` is passwordless for all users | Required for MELISA's jail shell to function without constant password prompts |
| Bash passthrough in the jail shell | Standard users need to run tools like `git`, `cargo`, etc. inside the MELISA session |
| `git safe.directory '*'` set globally | Prevents Git errors in multi-user shared repositories; acceptable trade-off for a controlled environment |
| `.env` files synced via Rsync outside Git | Environment secrets cannot be committed to Git; Rsync provides out-of-band transport |