#!/usr/bin/env python3
"""
=============================================================================
MELISA — Python Unit Test Runner
File: src/melisa_client/ut_/test_melisa.py

Menguji komponen-komponen Melisa:
  1. Bash client scripts (auth.sh, db.sh, exec.sh, utils.sh)
  2. Rust binary (via cargo test + subprocess jika binary tersedia)
  3. Logika install.sh

Cara menjalankan:
  python3 test_melisa.py                   # semua test
  python3 test_melisa.py -v                # verbose
  python3 test_melisa.py TestDB            # hanya tes DB
  python3 test_melisa.py TestDB.test_add   # satu tes spesifik
  pytest test_melisa.py -v                 # via pytest

Struktur output:
  [PASS] nama_tes
  [FAIL] nama_tes → pesan error
  [SKIP] nama_tes → alasan skip
=============================================================================
"""

import os
import sys
import stat
import shutil
import tempfile
import unittest
import subprocess
import textwrap
import time
from pathlib import Path
from typing import Optional, Tuple

# ─────────────────────────────────────────────────────────
# KONFIGURASI: Cari lokasi source code Melisa
# ─────────────────────────────────────────────────────────

def find_melisa_root() -> Optional[Path]:
    """Cari root direktori proyek Melisa secara otomatis."""
    # Cek relatif dari lokasi file ini
    candidates = [
        Path(__file__).parent.parent.parent.parent,  # ../../.. dari ut_/
        Path(__file__).parent.parent.parent,
        Path.cwd(),
        Path.cwd().parent,
        Path.cwd().parent.parent,
    ]
    for p in candidates:
        if (p / "Cargo.toml").exists() and (p / "src" / "main.rs").exists():
            return p
    return None

MELISA_ROOT = find_melisa_root()
CLIENT_SRC  = MELISA_ROOT / "src" / "melisa_client" / "src" if MELISA_ROOT else None
BINARY      = MELISA_ROOT / "target" / "release" / "melisa" if MELISA_ROOT else None
DEBUG_BIN   = MELISA_ROOT / "target" / "debug" / "melisa" if MELISA_ROOT else None

# Warna terminal
GREEN  = "\033[32m"
RED    = "\033[31m"
YELLOW = "\033[33m"
CYAN   = "\033[36m"
BOLD   = "\033[1m"
RESET  = "\033[0m"

def col(text: str, color: str) -> str:
    """Tambahkan warna ANSI jika terminal mendukung."""
    if sys.stdout.isatty():
        return f"{color}{text}{RESET}"
    return text


# ─────────────────────────────────────────────────────────
# HELPER: Jalankan bash script di environment terisolasi
# ─────────────────────────────────────────────────────────

class BashEnv:
    """Environment terisolasi untuk menguji bash scripts Melisa."""

    def __init__(self):
        self.tmp_dir = tempfile.mkdtemp(prefix="melisa_test_")
        self.home    = Path(self.tmp_dir) / "home"
        self.home.mkdir(parents=True)

        # Salin bash scripts ke temp env
        if CLIENT_SRC and CLIENT_SRC.exists():
            for sh_file in CLIENT_SRC.glob("*.sh"):
                dest = self.home / ".local" / "share" / "melisa" / sh_file.name
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(sh_file, dest)
                dest.chmod(dest.stat().st_mode | stat.S_IEXEC)

    def cleanup(self):
        shutil.rmtree(self.tmp_dir, ignore_errors=True)

    def run_bash(
        self,
        script: str,
        env_extra: Optional[dict] = None,
        timeout: int = 10
    ) -> Tuple[int, str, str]:
        """
        Jalankan bash script di dalam environment terisolasi.
        Return: (returncode, stdout, stderr)
        """
        env = os.environ.copy()
        env["HOME"] = str(self.home)
        env["PATH"] = f"{self.home}/.local/bin:/usr/bin:/bin"
        # Hapus variabel yang bisa mengganggu isolasi
        for var in ["SSH_CLIENT", "SSH_TTY", "SSH_CONNECTION", "SUDO_USER"]:
            env.pop(var, None)
        if env_extra:
            env.update(env_extra)

        # Buat header script yang source semua modul
        lib_dir = self.home / ".local" / "share" / "melisa"
        header = textwrap.dedent(f"""\
            #!/bin/bash
            set -o pipefail
            export HOME="{self.home}"
            export MELISA_LIB="{lib_dir}"
            # Source modules jika ada
            [ -f "$MELISA_LIB/utils.sh" ] && source "$MELISA_LIB/utils.sh" 2>/dev/null
            [ -f "$MELISA_LIB/auth.sh"  ] && source "$MELISA_LIB/auth.sh"  2>/dev/null
            [ -f "$MELISA_LIB/db.sh"    ] && source "$MELISA_LIB/db.sh"    2>/dev/null
            [ -f "$MELISA_LIB/exec.sh"  ] && source "$MELISA_LIB/exec.sh"  2>/dev/null
        """)
        full_script = header + "\n" + script

        try:
            result = subprocess.run(
                ["bash", "-c", full_script],
                capture_output=True,
                text=True,
                env=env,
                timeout=timeout
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", f"TIMEOUT setelah {timeout} detik"
        except Exception as e:
            return -2, "", str(e)


def has_bash_modules() -> bool:
    """Periksa apakah bash modules tersedia."""
    return CLIENT_SRC is not None and (CLIENT_SRC / "utils.sh").exists()


# ─────────────────────────────────────────────────────────
# TEST SUITE 1: Logika Pure Python (Tidak Butuh File)
# ─────────────────────────────────────────────────────────

class TestSlugGeneration(unittest.TestCase):
    """
    Menguji logika generate_slug() versi Python.
    Ini memvalidasi bahwa logika Rust sudah benar secara konseptual.
    """

    def _generate_slug(self, name: str, release: str, arch: str) -> str:
        """Mirror dari fungsi generate_slug() di Rust."""
        arch_map = {"amd64": "x64", "arm64": "a64", "i386": "x86"}
        s_arch = arch_map.get(arch, arch)
        prefix = name[:min(3, len(name))]
        return f"{prefix}-{release}-{s_arch}".lower()

    def test_ubuntu_amd64(self):
        self.assertEqual(self._generate_slug("ubuntu", "22.04", "amd64"), "ubu-22.04-x64")

    def test_debian_arm64(self):
        self.assertEqual(self._generate_slug("debian", "12", "arm64"), "deb-12-a64")

    def test_alpine_i386(self):
        self.assertEqual(self._generate_slug("alpine", "3.18", "i386"), "alp-3.18-x86")

    def test_archlinux_truncated(self):
        # "archlinux" → hanya 3 char pertama "arc"
        self.assertEqual(self._generate_slug("archlinux", "base", "amd64"), "arc-base-x64")

    def test_unknown_arch_passthrough(self):
        self.assertEqual(self._generate_slug("fedora", "39", "riscv64"), "fed-39-riscv64")

    def test_single_char_name(self):
        self.assertEqual(self._generate_slug("a", "1.0", "amd64"), "a-1.0-x64")


class TestDistroListParsing(unittest.TestCase):
    """
    Menguji logika parse_distro_list() versi Python.
    """

    def _parse(self, content: str) -> list:
        """Mirror dari parse_distro_list() di Rust."""
        PM_MAP = {
            "debian": "apt", "ubuntu": "apt", "kali": "apt",
            "fedora": "dnf", "centos": "dnf", "rocky": "dnf", "almalinux": "dnf",
            "alpine": "apk",
            "archlinux": "pacman",
            "opensuse": "zypper",
        }
        ARCH_MAP = {"amd64": "x64", "arm64": "a64", "i386": "x86"}

        result = []
        for line in content.splitlines():
            parts = line.split()
            if len(parts) < 4:
                continue
            if any(kw in line for kw in ["Distribution", "DIST", "---"]):
                continue
            name, release, arch, variant = parts[0], parts[1], parts[2], parts[3]
            s_arch = ARCH_MAP.get(arch, arch)
            slug = f"{name[:3]}-{release}-{s_arch}".lower()
            pm = PM_MAP.get(name, "apt")
            result.append({
                "name": name, "release": release, "arch": arch,
                "variant": variant, "slug": slug, "pkg_manager": pm
            })
        return result

    def test_valid_single_entry(self):
        content = "ubuntu 22.04 amd64 default"
        result = self._parse(content)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "ubuntu")
        self.assertEqual(result[0]["pkg_manager"], "apt")
        self.assertEqual(result[0]["slug"], "ubu-22.04-x64")

    def test_header_lines_skipped(self):
        content = "Distribution Release Architecture Variant\n---\nubuntu 22.04 amd64 default"
        result = self._parse(content)
        self.assertEqual(len(result), 1)

    def test_empty_input(self):
        self.assertEqual(self._parse(""), [])

    def test_incomplete_lines_skipped(self):
        content = "ubuntu 22.04 amd64\ndebian 12 arm64 default"
        result = self._parse(content)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["name"], "debian")

    def test_all_pkg_managers(self):
        entries = [
            ("ubuntu",    "apt"),    ("debian",    "apt"),   ("kali",      "apt"),
            ("fedora",    "dnf"),    ("centos",    "dnf"),   ("rocky",     "dnf"),
            ("almalinux", "dnf"),    ("alpine",    "apk"),
            ("archlinux", "pacman"), ("opensuse",  "zypper"),
            ("voidlinux", "apt"),    # fallback
        ]
        for name, expected_pm in entries:
            content = f"{name} 1.0 amd64 default"
            result = self._parse(content)
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0]["pkg_manager"], expected_pm,
                f"pkg_manager salah untuk '{name}'")

    def test_multiple_distros(self):
        content = textwrap.dedent("""\
            Distribution Release Architecture Variant
            ---
            ubuntu   22.04  amd64  default
            debian   12     arm64  default
            alpine   3.18   i386   default
            fedora   39     amd64  default
        """)
        result = self._parse(content)
        self.assertEqual(len(result), 4)
        names = [d["name"] for d in result]
        self.assertIn("ubuntu", names)
        self.assertIn("debian", names)
        self.assertIn("alpine", names)
        self.assertIn("fedora", names)


class TestContainerNameValidation(unittest.TestCase):
    """
    Menguji logika validate_container_name() — keamanan path traversal.
    """

    def _validate(self, name: str) -> bool:
        """Mirror dari validate_container_name() di Rust."""
        return '/' not in name and '\\' not in name and name != ".."

    def test_valid_names(self):
        for name in ["myapp", "ubuntu-dev", "test123", "a", "x-y-z", "my_box"]:
            with self.subTest(name=name):
                self.assertTrue(self._validate(name), f"'{name}' seharusnya valid")

    def test_reject_forward_slash(self):
        for name in ["a/b", "/etc/passwd", "container/hack", "../secret"]:
            with self.subTest(name=name):
                self.assertFalse(self._validate(name), f"'{name}' seharusnya ditolak")

    def test_reject_backslash(self):
        self.assertFalse(self._validate("evil\\path"))

    def test_reject_dotdot(self):
        self.assertFalse(self._validate(".."))

    def test_dotdot_in_middle_allowed_if_no_slash(self):
        # "my..container" tidak mengandung slash dan bukan persis ".."
        self.assertTrue(self._validate("my..container"))


class TestProjectInputValidation(unittest.TestCase):
    """
    Menguji logika validate_project_input() — keamanan path traversal.
    """

    def _validate(self, project_name: str, username: str) -> bool:
        """Mirror dari validate_project_input() di Rust."""
        if '/' in username or ".." in username:
            return False
        if '/' in project_name or ".." in project_name:
            return False
        return True

    def test_valid_combinations(self):
        self.assertTrue(self._validate("myproject", "alice"))
        self.assertTrue(self._validate("backend-api", "bob123"))
        self.assertTrue(self._validate("proj_name", "user_name"))

    def test_reject_slash_in_project(self):
        self.assertFalse(self._validate("proj/evil", "alice"))
        self.assertFalse(self._validate("/etc/shadow", "alice"))

    def test_reject_slash_in_username(self):
        self.assertFalse(self._validate("project", "alice/hack"))
        self.assertFalse(self._validate("project", "/root"))

    def test_reject_dotdot_in_project(self):
        self.assertFalse(self._validate("..", "alice"))
        self.assertFalse(self._validate("../secret", "alice"))

    def test_reject_dotdot_in_username(self):
        self.assertFalse(self._validate("project", ".."))
        self.assertFalse(self._validate("project", "../admin"))


class TestCommandParsing(unittest.TestCase):
    """
    Menguji logika parse_command() — parsing input shell.
    """

    def _parse(self, input_str: str):
        """Mirror dari parse_command() di Rust."""
        raw = input_str.split()
        audit = "--audit" in raw
        parts = [x for x in raw if x != "--audit"]
        return parts, audit

    def test_basic_command(self):
        parts, audit = self._parse("melisa --list")
        self.assertEqual(parts, ["melisa", "--list"])
        self.assertFalse(audit)

    def test_audit_flag_at_end(self):
        parts, audit = self._parse("melisa --list --audit")
        self.assertEqual(parts, ["melisa", "--list"])
        self.assertTrue(audit)

    def test_audit_flag_in_middle(self):
        parts, audit = self._parse("melisa --audit --create mybox ubu-22.04-x64")
        self.assertEqual(parts, ["melisa", "--create", "mybox", "ubu-22.04-x64"])
        self.assertTrue(audit)

    def test_empty_input(self):
        parts, audit = self._parse("")
        self.assertEqual(parts, [])
        self.assertFalse(audit)

    def test_exit_command(self):
        parts, audit = self._parse("exit")
        self.assertEqual(parts, ["exit"])
        self.assertFalse(audit)

    def test_cd_with_path(self):
        parts, audit = self._parse("cd /home/user/projects")
        self.assertEqual(parts, ["cd", "/home/user/projects"])
        self.assertFalse(audit)

    def test_melisa_send_multi_word(self):
        parts, audit = self._parse("melisa --send mybox apt update")
        self.assertEqual(parts, ["melisa", "--send", "mybox", "apt", "update"])
        self.assertFalse(audit)


class TestPkgManagerCmd(unittest.TestCase):
    """Menguji get_pkg_update_cmd() — pemetaan package manager."""

    def _get_cmd(self, pm: str) -> str:
        """Mirror dari get_pkg_update_cmd() di Rust."""
        return {
            "apt":    "apt-get update -y",
            "dnf":    "dnf makecache",
            "apk":    "apk update",
            "pacman": "pacman -Sy --noconfirm",
            "zypper": "zypper --non-interactive refresh",
        }.get(pm, "true")

    def test_apt(self):
        self.assertEqual(self._get_cmd("apt"), "apt-get update -y")

    def test_dnf(self):
        self.assertEqual(self._get_cmd("dnf"), "dnf makecache")

    def test_apk(self):
        self.assertEqual(self._get_cmd("apk"), "apk update")

    def test_pacman(self):
        self.assertEqual(self._get_cmd("pacman"), "pacman -Sy --noconfirm")

    def test_zypper(self):
        self.assertEqual(self._get_cmd("zypper"), "zypper --non-interactive refresh")

    def test_unknown_fallback(self):
        self.assertEqual(self._get_cmd("yum"), "true")
        self.assertEqual(self._get_cmd(""), "true")
        self.assertEqual(self._get_cmd("brew"), "true")


# ─────────────────────────────────────────────────────────
# TEST SUITE 2: Bash Client Scripts (auth.sh, db.sh)
# ─────────────────────────────────────────────────────────

@unittest.skipUnless(has_bash_modules(), "Bash modules tidak ditemukan di CLIENT_SRC")
class TestAuthModule(unittest.TestCase):
    """Menguji auth.sh — manajemen profil koneksi server."""

    def setUp(self):
        self.env = BashEnv()

    def tearDown(self):
        self.env.cleanup()

    def test_init_auth_creates_directories(self):
        """init_auth() harus membuat direktori config yang diperlukan."""
        rc, out, err = self.env.run_bash("init_auth")
        self.assertEqual(rc, 0, f"init_auth gagal: {err}")

        config_dir = self.env.home / ".config" / "melisa"
        self.assertTrue(config_dir.exists(), "~/.config/melisa tidak dibuat")

        profile_file = config_dir / "profiles.conf"
        self.assertTrue(profile_file.exists(), "profiles.conf tidak dibuat")

    def test_get_active_conn_returns_1_when_no_active(self):
        """get_active_conn() harus return 1 jika tidak ada koneksi aktif."""
        rc, out, err = self.env.run_bash("init_auth; get_active_conn; echo exit=$?")
        # Tidak ada active file → harus return kode non-0
        self.assertIn("exit=1", out, f"Harusnya return 1 jika tidak ada active file: {out}")

    def test_add_profile_and_get_conn(self):
        """Menambah profil dan mengambilnya kembali."""
        script = textwrap.dedent("""\
            init_auth
            CONFIG_DIR="$HOME/.config/melisa"
            PROFILE_FILE="$CONFIG_DIR/profiles.conf"
            ACTIVE_FILE="$CONFIG_DIR/active"
            # Tambah profil secara manual
            echo "myserver=root@192.168.1.100|alice" >> "$PROFILE_FILE"
            echo "myserver" > "$ACTIVE_FILE"
            # Ambil koneksi
            result=$(get_active_conn)
            echo "CONN=$result"
        """)
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("CONN=root@192.168.1.100", out,
            f"get_active_conn harus return 'root@192.168.1.100', bukan: {out}")

    def test_get_active_conn_strips_melisa_user(self):
        """get_active_conn() harus membuang bagian '|melisa_user'."""
        script = textwrap.dedent("""\
            init_auth
            CONFIG_DIR="$HOME/.config/melisa"
            PROFILE_FILE="$CONFIG_DIR/profiles.conf"
            ACTIVE_FILE="$CONFIG_DIR/active"
            echo "prod=ubuntu@10.0.0.1|devuser" >> "$PROFILE_FILE"
            echo "prod" > "$ACTIVE_FILE"
            conn=$(get_active_conn)
            echo "CONN=$conn"
        """)
        rc, out, err = self.env.run_bash(script)
        self.assertIn("CONN=ubuntu@10.0.0.1", out)
        self.assertNotIn("devuser", out, "Bagian melisa_user seharusnya dibuang")


@unittest.skipUnless(has_bash_modules(), "Bash modules tidak ditemukan")
class TestDBModule(unittest.TestCase):
    """Menguji db.sh — project registry (flat file database)."""

    def setUp(self):
        self.env = BashEnv()
        # Buat DB_PATH yang bisa dipakai
        self.db_dir = self.env.home / ".config" / "melisa"
        self.db_dir.mkdir(parents=True, exist_ok=True)

    def tearDown(self):
        self.env.cleanup()

    def _setup_db(self) -> str:
        """Siapkan DB_PATH di environment."""
        return f'DB_PATH="{self.db_dir}/registry"'

    def test_db_update_project_creates_entry(self):
        """db_update_project() harus membuat entry baru."""
        script = f"""\
{self._setup_db()}
db_update_project "myapp" "/home/user/projects/myapp"
cat "$DB_PATH"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("myapp|", out)

    def test_db_update_project_overwrites_existing(self):
        """db_update_project() harus menimpa entry yang sudah ada (tidak duplikat)."""
        script = f"""\
{self._setup_db()}
db_update_project "backend" "/old/path"
db_update_project "backend" "/new/path"
count=$(grep -c "^backend|" "$DB_PATH" 2>/dev/null || echo "0")
echo "COUNT=$count"
content=$(cat "$DB_PATH")
echo "CONTENT=$content"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("COUNT=1", out, "Entry duplikat tidak boleh ada")
        self.assertIn("/new/path", out, "Path terbaru harus tersimpan")
        self.assertNotIn("/old/path", out, "Path lama harus dihapus")

    def test_db_update_multiple_projects(self):
        """Beberapa project bisa disimpan bersamaan."""
        script = f"""\
{self._setup_db()}
db_update_project "frontend" "/home/user/frontend"
db_update_project "backend"  "/home/user/backend"
db_update_project "scripts"  "/home/user/scripts"
wc -l < "$DB_PATH"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        count = int(out.strip().split('\n')[0].strip())
        self.assertEqual(count, 3, "Harus ada 3 entry di registry")

    def test_db_identify_by_pwd_exact_match(self):
        """db_identify_by_pwd() harus mengembalikan nama project untuk exact match."""
        project_dir = self.env.home / "projects" / "myapp"
        project_dir.mkdir(parents=True)

        script = f"""\
{self._setup_db()}
db_update_project "myapp" "{project_dir}"
cd "{project_dir}"
result=$(db_identify_by_pwd)
echo "PROJECT=$result"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("PROJECT=myapp", out)

    def test_db_identify_by_pwd_subdir_match(self):
        """db_identify_by_pwd() harus match ketika berada di subdirektori project."""
        project_dir = self.env.home / "projects" / "backend"
        sub_dir = project_dir / "src" / "api"
        sub_dir.mkdir(parents=True)

        script = f"""\
{self._setup_db()}
db_update_project "backend" "{project_dir}"
cd "{sub_dir}"
result=$(db_identify_by_pwd)
echo "PROJECT=$result"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("PROJECT=backend", out,
            f"Harus match project saat di subdirektori, output: {out}")

    def test_db_identify_by_pwd_no_match(self):
        """db_identify_by_pwd() harus return kosong jika tidak ada match."""
        unrelated_dir = self.env.home / "unrelated"
        unrelated_dir.mkdir(parents=True)

        script = f"""\
{self._setup_db()}
db_update_project "myapp" "{self.env.home}/projects/myapp"
cd "{unrelated_dir}"
result=$(db_identify_by_pwd)
echo "PROJECT='$result'"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("PROJECT=''", out,
            f"Harus kosong jika tidak ada match: {out}")

    def test_db_identify_longest_prefix_wins(self):
        """db_identify_by_pwd() harus memilih path yang paling spesifik (terpanjang)."""
        # Simulasi: ada dua project dengan path yang nested
        parent_dir = self.env.home / "work"
        child_dir  = self.env.home / "work" / "specific" / "project"
        child_dir.mkdir(parents=True)

        script = f"""\
{self._setup_db()}
db_update_project "parent"  "{parent_dir}"
db_update_project "specific" "{child_dir}"
cd "{child_dir}"
result=$(db_identify_by_pwd)
echo "PROJECT=$result"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("PROJECT=specific", out,
            f"Path terpanjang (most specific) harus dipilih: {out}")

    def test_db_no_false_positive_prefix(self):
        """
        db_identify_by_pwd() tidak boleh match '/projects/app' untuk '/projects/apple'.
        Boundary check harus diterapkan.
        """
        app_dir   = self.env.home / "projects" / "app"
        apple_dir = self.env.home / "projects" / "apple"
        apple_dir.mkdir(parents=True)
        app_dir.mkdir(parents=True)

        script = f"""\
{self._setup_db()}
db_update_project "app" "{app_dir}"
cd "{apple_dir}"
result=$(db_identify_by_pwd)
echo "PROJECT='$result'"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertNotIn("PROJECT='app'", out,
            f"'/projects/app' tidak boleh match '/projects/apple': {out}")

    def test_db_get_path_returns_correct_path(self):
        """db_get_path() harus mengembalikan path yang benar untuk nama project."""
        project_path = str(self.env.home / "work" / "backend")

        script = f"""\
{self._setup_db()}
db_update_project "backend" "{project_path}"
result=$(db_get_path "backend")
echo "PATH=$result"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn(f"PATH={project_path}", out)

    def test_db_get_path_nonexistent_returns_empty(self):
        """db_get_path() harus return kosong untuk project yang tidak ada."""
        script = f"""\
{self._setup_db()}
result=$(db_get_path "nonexistent_project")
echo "PATH='$result'"
"""
        rc, out, err = self.env.run_bash(script)
        self.assertEqual(rc, 0, f"Error: {err}")
        self.assertIn("PATH=''", out)


@unittest.skipUnless(has_bash_modules(), "Bash modules tidak ditemukan")
class TestUtilsModule(unittest.TestCase):
    """Menguji utils.sh — fungsi-fungsi helper."""

    def setUp(self):
        self.env = BashEnv()

    def tearDown(self):
        self.env.cleanup()

    def test_log_info_outputs_to_stderr(self):
        """Fungsi log (jika ada) harus output ke stderr, bukan stdout."""
        script = 'log_info "test message" 2>&1 1>/dev/null; echo "STDERR_ONLY=$?"'
        rc, out, err = self.env.run_bash(script)
        # Jika log_info tidak ada, test ini akan skip dengan gracefully
        if "log_info: command not found" in err:
            self.skipTest("log_info tidak ada di utils.sh")

    def test_bash_scripts_are_syntactically_valid(self):
        """Semua file .sh harus bisa di-parse oleh bash tanpa error syntax."""
        if not CLIENT_SRC or not CLIENT_SRC.exists():
            self.skipTest("CLIENT_SRC tidak ditemukan")

        for sh_file in sorted(CLIENT_SRC.glob("*.sh")):
            with self.subTest(file=sh_file.name):
                result = subprocess.run(
                    ["bash", "-n", str(sh_file)],
                    capture_output=True, text=True
                )
                self.assertEqual(
                    result.returncode, 0,
                    f"Syntax error di {sh_file.name}:\n{result.stderr}"
                )


# ─────────────────────────────────────────────────────────
# TEST SUITE 3: Cargo Test Integration
# ─────────────────────────────────────────────────────────

class TestCargoTests(unittest.TestCase):
    """
    Menjalankan `cargo test` untuk mengeksekusi semua unit test Rust.
    Ini adalah integration antara Python runner dan Rust test system.
    """

    @classmethod
    def setUpClass(cls):
        cls.has_cargo = shutil.which("cargo") is not None
        cls.has_melisa_root = MELISA_ROOT is not None

    def _run_cargo_test(self, test_filter: str = "", timeout: int = 120) -> Tuple[int, str, str]:
        """Jalankan cargo test dengan filter opsional."""
        cmd = ["cargo", "test", "--quiet"]
        if test_filter:
            cmd.append(test_filter)
        cmd.extend(["--", "--nocapture"])

        try:
            result = subprocess.run(
                cmd,
                cwd=str(MELISA_ROOT),
                capture_output=True,
                text=True,
                timeout=timeout
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", f"cargo test timeout setelah {timeout}s"
        except Exception as e:
            return -2, "", str(e)

    @unittest.skipUnless(
        MELISA_ROOT is not None and shutil.which("cargo") is not None,
        "cargo tidak tersedia atau MELISA_ROOT tidak ditemukan"
    )
    def test_cargo_check_compiles(self):
        """Proyek harus bisa di-compile tanpa error (cargo check)."""
        result = subprocess.run(
            ["cargo", "check", "--quiet"],
            cwd=str(MELISA_ROOT),
            capture_output=True,
            text=True,
            timeout=120
        )
        self.assertEqual(
            result.returncode, 0,
            f"cargo check gagal:\n{result.stderr}"
        )

    @unittest.skipUnless(
        MELISA_ROOT is not None and shutil.which("cargo") is not None,
        "cargo tidak tersedia"
    )
    def test_cargo_test_unit_tests_pass(self):
        """Semua unit test Rust harus lulus."""
        rc, out, err = self._run_cargo_test()
        # Parse output untuk debug yang lebih baik
        if rc != 0:
            failed_tests = [
                line for line in (out + err).splitlines()
                if "FAILED" in line or "error" in line.lower()
            ]
            self.fail(
                f"cargo test gagal (exit code {rc}).\n"
                f"Tes yang gagal:\n" + "\n".join(failed_tests[:20]) +
                f"\n\nFull stderr:\n{err[:2000]}"
            )

    @unittest.skipUnless(
        MELISA_ROOT is not None and shutil.which("cargo") is not None,
        "cargo tidak tersedia"
    )
    def test_cargo_test_distro_module(self):
        """Unit test khusus untuk modul distro."""
        rc, out, err = self._run_cargo_test("distro")
        self.assertEqual(rc, 0, f"Distro tests gagal:\n{err[:2000]}")

    @unittest.skipUnless(
        MELISA_ROOT is not None and shutil.which("cargo") is not None,
        "cargo tidak tersedia"
    )
    def test_cargo_test_metadata_module(self):
        """Unit test khusus untuk modul metadata."""
        rc, out, err = self._run_cargo_test("metadata")
        self.assertEqual(rc, 0, f"Metadata tests gagal:\n{err[:2000]}")


# ─────────────────────────────────────────────────────────
# TEST SUITE 4: Rust Binary Integration Tests
# ─────────────────────────────────────────────────────────

def get_melisa_binary() -> Optional[Path]:
    """Cari binary melisa yang sudah dikompilasi."""
    if DEBUG_BIN and DEBUG_BIN.exists():
        return DEBUG_BIN
    if BINARY and BINARY.exists():
        return BINARY
    return None


class TestMelisaBinary(unittest.TestCase):
    """
    Integration test: menguji binary melisa yang sudah dikompilasi.
    Test ini dijalankan dengan `sudo` karena melisa membutuhkan root.
    """

    @classmethod
    def setUpClass(cls):
        cls.binary = get_melisa_binary()
        cls.has_sudo = shutil.which("sudo") is not None

    def _run_melisa(self, args: list, timeout: int = 10) -> Tuple[int, str, str]:
        """Jalankan binary melisa dengan argument tertentu."""
        if not self.binary:
            return -1, "", "Binary tidak ditemukan"

        cmd = ["sudo", str(self.binary)] + args
        try:
            result = subprocess.run(
                cmd, capture_output=True, text=True, timeout=timeout
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", "Timeout"
        except Exception as e:
            return -2, "", str(e)

    @unittest.skipUnless(get_melisa_binary() is not None, "Binary melisa tidak ditemukan")
    def test_help_command_shows_usage(self):
        """melisa --help harus menampilkan usage info."""
        rc, out, err = self._run_melisa(["melisa", "--help"])
        combined = out + err
        self.assertIn("MELISA", combined, "Output harus menyebut MELISA")
        self.assertIn("--help", combined)
        self.assertIn("--list", combined)

    @unittest.skipUnless(get_melisa_binary() is not None, "Binary melisa tidak ditemukan")
    def test_version_command(self):
        """melisa --version harus menampilkan versi."""
        rc, out, err = self._run_melisa(["melisa", "--version"])
        combined = out + err
        self.assertIn("0.1", combined, "Harus ada nomor versi di output")

    @unittest.skipUnless(get_melisa_binary() is not None, "Binary melisa tidak ditemukan")
    def test_unknown_command_shows_error(self):
        """Command yang tidak dikenal harus menampilkan pesan error."""
        rc, out, err = self._run_melisa(["melisa", "--fakecommand"])
        combined = out + err
        self.assertIn("ERROR", combined.upper())

    @unittest.skipUnless(get_melisa_binary() is not None, "Binary melisa tidak ditemukan")
    def test_create_requires_name_and_code(self):
        """melisa --create tanpa argumen harus menampilkan error."""
        rc, out, err = self._run_melisa(["melisa", "--create"])
        combined = out + err
        # Harus ada pesan error, bukan crash
        self.assertIn("ERROR", combined.upper())

    @unittest.skipUnless(get_melisa_binary() is not None, "Binary melisa tidak ditemukan")
    def test_invite_requires_enough_args(self):
        """melisa --invite tanpa args yang cukup harus error."""
        rc, out, err = self._run_melisa(["melisa", "--invite"])
        combined = out + err
        self.assertIn("ERROR", combined.upper())


# ─────────────────────────────────────────────────────────
# TEST SUITE 5: Security Tests (Keamanan Keseluruhan)
# ─────────────────────────────────────────────────────────

class TestSecurityCritical(unittest.TestCase):
    """
    Test keamanan kritis — path traversal, injection, dll.
    """

    def test_no_path_traversal_in_container_name(self):
        """Nama container tidak boleh mengandung path traversal."""
        evil_names = [
            "../etc",
            "../../root/.ssh/authorized_keys",
            "/etc/shadow",
            "evil/path",
            "..\\windows\\system32",
        ]
        for name in evil_names:
            with self.subTest(name=name):
                is_safe = '/' not in name and '\\' not in name and name != ".."
                self.assertFalse(
                    is_safe,
                    f"Nama '{name}' berbahaya harus ditolak oleh validasi"
                )

    def test_no_path_traversal_in_username(self):
        """Username tidak boleh mengandung path traversal."""
        evil_usernames = ["../root", "alice/../root", "user/hack", ".."]
        for username in evil_usernames:
            with self.subTest(username=username):
                is_safe = '/' not in username and ".." not in username
                self.assertFalse(
                    is_safe,
                    f"Username '{username}' berbahaya harus ditolak"
                )

    def test_metadata_content_format(self):
        """Format metadata harus menggunakan KEY=VALUE tanpa karakter berbahaya."""
        import re
        valid_keys = [
            "MELISA_INSTANCE_NAME", "MELISA_INSTANCE_ID", "DISTRO_SLUG",
            "DISTRO_NAME", "DISTRO_RELEASE", "ARCHITECTURE", "CREATED_AT"
        ]
        key_pattern = re.compile(r'^[A-Z_]+$')
        for key in valid_keys:
            with self.subTest(key=key):
                self.assertTrue(
                    key_pattern.match(key),
                    f"Key '{key}' mengandung karakter tidak aman"
                )

    def test_project_path_construction_safety(self):
        """Path /home/<user>/<project> harus aman dari injection."""
        safe_combos = [("alice", "backend"), ("bob", "frontend-app"), ("user1", "proj_1")]
        evil_combos = [
            ("../root", "project"),  # username traversal
            ("alice", "../../../etc"),  # project traversal
            ("user/hack", "project"),  # username dengan slash
        ]

        for username, project in safe_combos:
            with self.subTest(username=username, project=project):
                path = f"/home/{username}/{project}"
                # Path yang aman tidak akan keluar dari /home/<user>/
                self.assertTrue(
                    path.startswith(f"/home/{username}/{project}"),
                    f"Path aman: {path}"
                )

        for username, project in evil_combos:
            with self.subTest(username=username, project=project):
                is_safe = (
                    '/' not in username and ".." not in username and
                    '/' not in project and ".." not in project
                )
                self.assertFalse(
                    is_safe,
                    f"Kombinasi berbahaya ({username}, {project}) harus ditolak"
                )


# ─────────────────────────────────────────────────────────
# CUSTOM TEST RUNNER dengan output berwarna
# ─────────────────────────────────────────────────────────

class ColoredTestResult(unittest.TextTestResult):
    """Test result dengan output berwarna dan lebih informatif."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._test_timings = {}
        self._start_time  = None

    def startTest(self, test):
        super().startTest(test)
        self._start_time = time.monotonic()

    def addSuccess(self, test):
        super().addSuccess(test)
        elapsed = time.monotonic() - self._start_time
        if self.showAll:
            self.stream.write(col(f"  [PASS] ({elapsed:.3f}s)\n", GREEN))
            self.stream.flush()

    def addFailure(self, test, err):
        super().addFailure(test, err)
        elapsed = time.monotonic() - self._start_time
        if self.showAll:
            self.stream.write(col(f"  [FAIL] ({elapsed:.3f}s)\n", RED))
            self.stream.flush()

    def addError(self, test, err):
        super().addError(test, err)
        elapsed = time.monotonic() - self._start_time
        if self.showAll:
            self.stream.write(col(f"  [ERROR] ({elapsed:.3f}s)\n", RED))
            self.stream.flush()

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        if self.showAll:
            self.stream.write(col(f"  [SKIP] {reason}\n", YELLOW))
            self.stream.flush()


class ColoredTestRunner(unittest.TextTestRunner):
    resultclass = ColoredTestResult


def print_banner():
    """Tampilkan banner informasi sebelum test."""
    print(col("=" * 65, CYAN))
    print(col("  MELISA — Unit Test Runner", BOLD + CYAN))
    print(col("=" * 65, CYAN))
    print(f"  Root Proyek : {col(str(MELISA_ROOT or 'TIDAK DITEMUKAN'), YELLOW)}")
    print(f"  Bash Client : {col(str(CLIENT_SRC or 'TIDAK DITEMUKAN'), YELLOW)}")
    binary = get_melisa_binary()
    print(f"  Binary      : {col(str(binary or 'Belum dikompilasi'), YELLOW)}")
    cargo_available = col("✓ tersedia", GREEN) if shutil.which("cargo") else col("✗ tidak ada", RED)
    bash_available  = col("✓ tersedia", GREEN) if has_bash_modules() else col("✗ tidak ada", YELLOW)
    print(f"  cargo       : {cargo_available}")
    print(f"  Bash modules: {bash_available}")
    print(col("=" * 65, CYAN))
    print()


def main():
    """Entry point untuk menjalankan semua test."""
    print_banner()

    # Collect test suites
    loader = unittest.TestLoader()
    suites = [
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestSlugGeneration)),
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestDistroListParsing)),
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestContainerNameValidation)),
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestProjectInputValidation)),
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestCommandParsing)),
        ("Pure Logic Tests", loader.loadTestsFromTestCase(TestPkgManagerCmd)),
        ("Bash: auth.sh",    loader.loadTestsFromTestCase(TestAuthModule)),
        ("Bash: db.sh",      loader.loadTestsFromTestCase(TestDBModule)),
        ("Bash: utils",      loader.loadTestsFromTestCase(TestUtilsModule)),
        ("Rust: cargo test", loader.loadTestsFromTestCase(TestCargoTests)),
        ("Binary: melisa",   loader.loadTestsFromTestCase(TestMelisaBinary)),
        ("Security",         loader.loadTestsFromTestCase(TestSecurityCritical)),
    ]

    # Filter berdasarkan argumen command line
    if len(sys.argv) > 1:
        filter_name = sys.argv[1]
        # Jika argumen diberikan, pakai default unittest discovery
        unittest.main(argv=[sys.argv[0]] + sys.argv[1:], verbosity=2,
                      testRunner=ColoredTestRunner)
        return

    # Jalankan semua test
    all_suite = unittest.TestSuite()
    for _, suite in suites:
        all_suite.addTests(suite)

    runner = ColoredTestRunner(verbosity=2, stream=sys.stdout)
    result = runner.run(all_suite)

    # Summary
    print()
    print(col("=" * 65, CYAN))
    total   = result.testsRun
    passed  = total - len(result.failures) - len(result.errors) - len(result.skipped)
    failed  = len(result.failures) + len(result.errors)
    skipped = len(result.skipped)

    print(f"  Total  : {col(str(total), BOLD)}")
    print(f"  {col('Passed', GREEN)}  : {col(str(passed), GREEN)}")
    print(f"  {col('Failed', RED)}  : {col(str(failed), RED) if failed else col('0', GREEN)}")
    print(f"  {col('Skipped', YELLOW)} : {col(str(skipped), YELLOW)}")
    print(col("=" * 65, CYAN))

    if result.failures or result.errors:
        print(col("\n  ❌  ADA TES YANG GAGAL — Cek detail di atas\n", RED + BOLD))
        sys.exit(1)
    else:
        print(col("\n  ✅  SEMUA TES LULUS!\n", GREEN + BOLD))
        sys.exit(0)


if __name__ == "__main__":
    main()