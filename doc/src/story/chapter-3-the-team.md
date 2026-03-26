# Chapter III: The Team Assembles

*Or: How to give thirty people identical rooms without losing your mind.*

---

Three days after Alice's first login, Bob shows up.

Bob is a frontend developer. He moves fast, breaks things, and then messages Erick at 11 PM saying the thing is broken. Erick has worked with Bob for two years. He respects Bob's output — the interfaces Bob ships are genuinely good — but he has rebuilt Bob's laptop environment from scratch on three separate occasions.

Bob does not get a container. Bob gets two containers. One for stable work. One for experiments. Erick has learned.

```
melisa@saferoom-01:~> melisa --create bob-stable ubu-jammy-x64
melisa@saferoom-01:~> melisa --create bob-lab debian-bookworm-x64
melisa@saferoom-01:~> melisa --run bob-stable
melisa@saferoom-01:~> melisa --run bob-lab
melisa@saferoom-01:~> melisa --add bob
```

Bob's onboarding is identical to Alice's. Same commands. Different containers. Two minutes.

---

## The Project

Bob and Alice are building the same application. This is the part that gets complicated without a system like MELISA.

Erick creates the shared project:

```
melisa@saferoom-01:~> melisa --new_project backend-api
```

Behind the scenes: a bare Git repository appears at `/opt/melisa/projects/backend-api/` with a `post-receive` hook that will automatically propagate any push to all team members' working directories.

He invites the team:

```
melisa@saferoom-01:~> melisa --invite backend-api alice bob
```

MELISA runs `git clone` from the master into `/home/alice/backend-api/` and `/home/bob/backend-api/`. Both users now have working copies. The hook is live.

Alice logs in, checks her projects:

```
melisa@alice:~> melisa --projects
```

```
[INFO] Scanning workspace for active projects...
  - backend-api   (/home/alice/backend-api)
```

She navigates to the project, makes a change, and pushes:

```
melisa@alice:~> cd ~/backend-api
melisa@alice:~/backend-api> git add . && git push origin master
```

The `post-receive` hook fires. Somewhere in the background:

```bash
sudo melisa --update-all backend-api
```

Bob's `/home/bob/backend-api/` is instantly updated. He didn't pull. He didn't ask. It just happened.

---

## Carol's Problem

Carol is not a developer. Carol is a teacher.

She has thirty students. They all need identical Python development environments by Monday morning for a machine learning workshop. The environments need to be isolated (students tend to `pip install` random things with creative spellings), identical (the workshop materials assume a specific Python version and set of libraries), and reproducible (if a student breaks their environment, it needs to be restorable in under a minute without wasting class time).

The previous solution was Docker Desktop on each student's laptop. This took an entire Friday to set up, required IT to install the tool on the university's locked-down machines, and failed on four laptops due to virtualization flags in the BIOS.

Carol emails Erick. Erick says: *"Give me an hour."*

---

## The Classroom

Erick looks at his container list. Thirty environments is a loop:

```bash
for i in $(seq 1 30); do
  melisa --create student-$i ubu-jammy-x64
  melisa --run student-$i
  melisa --send student-$i apt-get install -y python3 python3-pip
  melisa --send student-$i pip3 install numpy pandas scikit-learn matplotlib jupyter
done
```

He doesn't actually type this in the interactive shell — he runs it via SSH from his workstation. The Bash passthrough in MELISA means `for` loops work. Each iteration runs `--send` to execute commands inside the container non-interactively, streaming the output back.

Thirty containers. Thirty identical environments. Fifty-four minutes.

Then he creates thirty user accounts. Another loop. Two minutes.

```
melisa@saferoom-01:~> melisa --user
```

Thirty-three entries. Alice. Bob. Thirty students. The sudoers directory has thirty-three corresponding files. Everything is accounted for.

Carol's students log in with the credentials Erick sends them. They type:

```
melisa@student-07:~> melisa --use student-7
```

```
root@student-7:/#
```

They're inside. Python is there. Jupyter is there. The machine learning libraries are there. The classroom starts on time.

---

## The First Breakage

This happens on day two of the workshop.

A student — student-14 — decides to upgrade Python. This is a reasonable idea in the abstract and a catastrophic idea in the specific: they ran `apt-get remove python3` trying to reinstall a newer version, and the package manager removed half the system libraries in the process.

In the old world, this would mean a two-hour re-imaging session, possibly losing the student's work, and delaying the rest of the class.

In the MELISA world, Erick gets the message at 10:15 AM. By 10:17 AM:

```
melisa@saferoom-01:~> melisa --delete student-14
Are you sure you want to permanently delete 'student-14'? (y/N): y

[SUCCESS] Container 'student-14' destroyed.

melisa@saferoom-01:~> melisa --create student-14 ubu-jammy-x64
melisa@saferoom-01:~> melisa --run student-14
melisa@saferoom-01:~> melisa --send student-14 apt-get install -y python3 python3-pip
melisa@saferoom-01:~> melisa --send student-14 pip3 install numpy pandas scikit-learn matplotlib jupyter
```

The student's environment is restored. Their project files were never in the container — Erick had them storing work in `/home/student-14/` on the host, which is unaffected by container deletion. They lose nothing.

The class continues.

---

## What Just Happened (The Dry Version)

Three different use cases, all resolved with the same set of commands:

**Individual developer onboarding:** `--create`, `--run`, `--add`. Under two minutes. The new user gets a container they own and can break freely without touching anyone else's environment.

**Team collaboration:** `--new_project`, `--invite`. A shared Git repository with automatic synchronization. No merge conflicts from incomplete pulls. No "it works on my machine" because everyone's machine is the same machine.

**Classroom at scale:** The same commands in a loop. Thirty environments provisioned in under an hour. One breakage resolved in two minutes without data loss.

MELISA doesn't care whether you have two users or thirty. The operations are identical. The time scales linearly with container provisioning, not with complexity.

**Next:** [Chapter IV — The Workflow](./chapter-4-the-workflow.md)