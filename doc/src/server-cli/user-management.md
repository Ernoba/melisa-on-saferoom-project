# User Management Commands

## `--add <username>`

**Access:** Administrator only

Provisions a new MELISA user. This is an **interactive command** — it prompts for the user's role and password.

```
melisa@host:~> melisa --add alice
```

**Interactive flow:**

```
--- Provisioning New MELISA User: alice ---
Select Access Level for alice:
  1) Administrator (Full Management: Users, Projects, & LXC)
  2) Standard User (Project & LXC Management Only)
Enter choice (1/2): 2

[ACTION] Please set the authentication password for alice:
New password:
Retype new password:
passwd: password updated successfully
[SUCCESS] User account 'alice' successfully created.
[SUCCESS] Privilege configuration deployed successfully.
```

**What `--add` does step by step:**

1. Prompts for role selection (1 = Admin, 2 = Standard; any other input defaults to Standard)
2. Creates the Linux system user with MELISA as the login shell:
   ```bash
   useradd -m -s /usr/local/bin/melisa alice
   ```
3. Sets home directory permissions to `700` (private)
4. Launches `passwd alice` interactively for password setup
5. Generates and deploys the custom sudoers file at `/etc/sudoers.d/melisa_alice` with permissions `0440`

---

## `--remove <username>`

**Access:** Administrator only

De-provisions a MELISA user. Requires interactive confirmation:

```
melisa@host:~> melisa --remove alice
Are you sure you want to permanently delete user 'alice'? (y/N): y
```

**What `--remove` does step by step:**

1. Prompts for confirmation (`y`/`yes` to proceed; empty input or any other value aborts)
2. Sends `SIGKILL` to all processes owned by Alice: `pkill -u alice`
   - This prevents "device or resource busy" errors when the home directory is deleted
3. Deletes the user and home directory: `userdel -r -f alice`
4. Removes the custom sudoers file: `rm -f /etc/sudoers.d/melisa_alice`

**Confirmation is mandatory.** Pressing Enter without input aborts safely:
```
[CANCEL] No input detected. User deletion aborted.
```

> **Note:** If Alice has project clones in her home directory (`/home/alice/myapp/`), those are deleted with her home. The master repository at `/opt/melisa/projects/myapp/` is unaffected.

---

## `--passwd <username>`

**Access:** All users (for themselves) · Admins (for any user)

Updates the authentication password for a MELISA user:

```
melisa@host:~> melisa --passwd alice
[ACTION] Please set the authentication password for alice:
New password:
Retype new password:
passwd: password updated successfully
[SUCCESS] Password successfully updated for alice.
```

Launches `sudo passwd <username>` interactively. Returns `true` (success) or `false` (failure) to the calling code — used internally by `--add` to conditionally deploy sudoers only if password setup succeeded.

---

## `--upgrade <username>`

**Access:** Administrator only

Elevates an existing Standard User to Administrator role by rewriting their sudoers configuration with the full admin command set:

```
melisa@host:~> melisa --upgrade alice
```

After upgrade, Alice's `/etc/sudoers.d/melisa_alice` is replaced with the extended admin policy that includes `useradd`, `userdel`, `passwd`, `chmod`, `chown`, and all other admin-only commands.

> **Note:** There is currently no downgrade command. To revoke admin privileges, delete and recreate the user.

---

## `--user`

**Access:** All users (Admins see full list; standard users see their own context)

Lists all registered MELISA users on the host:

```
melisa@host:~> melisa --user
```

Internally reads the sudoers directory (`/etc/sudoers.d/`) for files matching `melisa_*` and cross-references them with the system user database to build the user list.

---

## `--clean`

**Access:** Administrator only

Scans `/etc/sudoers.d/` for orphaned MELISA sudoers files — files named `melisa_<username>` where `<username>` no longer exists as a system user.

```
melisa@host:~> melisa --clean
[INFO] Scanning for orphaned sudoers configurations...
[SUCCESS] Removed orphaned file: /etc/sudoers.d/melisa_ghost_user
[INFO] Cleanup complete. No more orphaned configurations found.
```

**When to use:** Run `--clean` after manually deleting users outside MELISA, or after a system migration where the user database was rebuilt.

---

## User Management Summary

```bash
# Full lifecycle example:

# Create standard user
melisa --add alice

# Create admin user (choose option 1 at prompt)
melisa --add bob

# Promote alice to admin
melisa --upgrade alice

# Change alice's password
melisa --passwd alice

# List all users
melisa --user

# Delete bob
melisa --remove bob

# Clean up any ghost sudoers files
melisa --clean
```