#!/usr/bin/env python3
"""
run_tests.py — Quick launcher untuk test suite Melisa.

Cara pakai:
  python3 run_tests.py              # semua test
  python3 run_tests.py --rust       # hanya cargo test
  python3 run_tests.py --bash       # hanya bash tests
  python3 run_tests.py --logic      # hanya pure logic tests
  python3 run_tests.py --security   # hanya security tests
  python3 run_tests.py --debug      # tampilkan lebih banyak info
"""

import sys
import os
import subprocess
from pathlib import Path

# Tambahkan direktori test ke PYTHONPATH
test_dir = Path(__file__).parent / "src" / "melisa_client" / "ut_"
sys.path.insert(0, str(test_dir))


def print_section(title: str):
    print(f"\n\033[1;36m{'─' * 55}\033[0m")
    print(f"\033[1;36m  {title}\033[0m")
    print(f"\033[1;36m{'─' * 55}\033[0m\n")


def run_quick_sanity_check():
    """Jalankan sanity check cepat sebelum test utama."""
    print_section("SANITY CHECK")

    checks = []

    # 1. Python version
    v = sys.version_info
    ok = v.major == 3 and v.minor >= 8
    checks.append(("Python 3.8+", ok, f"Python {v.major}.{v.minor}.{v.micro}"))

    # 2. Bash tersedia
    import shutil
    ok = shutil.which("bash") is not None
    checks.append(("bash", ok, shutil.which("bash") or "tidak ditemukan"))

    # 3. cargo tersedia
    ok = shutil.which("cargo") is not None
    checks.append(("cargo (Rust)", ok, shutil.which("cargo") or "tidak ditemukan"))

    # 4. Cargo.toml ada
    root = Path(__file__).parent
    ok = (root / "Cargo.toml").exists()
    checks.append(("Cargo.toml", ok, str(root / "Cargo.toml") if ok else "tidak ditemukan"))

    # 5. Bash client scripts
    client_src = root / "src" / "melisa_client" / "src"
    has_scripts = any(client_src.glob("*.sh")) if client_src.exists() else False
    checks.append(("Bash scripts", has_scripts,
                   str(client_src) if has_scripts else "tidak ada .sh di client/src"))

    for name, status, info in checks:
        icon  = "\033[32m✓\033[0m" if status else "\033[33m⚠\033[0m"
        color = "\033[32m" if status else "\033[33m"
        print(f"  {icon}  {name:<20} {color}{info}\033[0m")

    print()
    return all(status for _, status, _ in checks[:2])  # Python dan bash wajib


def main():
    args = set(sys.argv[1:])
    debug = "--debug" in args or "-d" in args

    # Sanity check
    if not run_quick_sanity_check():
        print("\033[31m[FATAL] Prerequisite dasar tidak terpenuhi.\033[0m")
        sys.exit(1)

    # Tentukan test yang akan dijalankan
    pytest_available = subprocess.run(
        [sys.executable, "-m", "pytest", "--version"],
        capture_output=True
    ).returncode == 0

    test_file = str(test_dir / "test_melisa.py")

    if pytest_available:
        cmd = [sys.executable, "-m", "pytest", test_file, "-v", "--tb=short"]

        if "--rust" in args:
            cmd += ["-k", "TestCargo"]
        elif "--bash" in args:
            cmd += ["-k", "TestAuth or TestDB or TestUtils"]
        elif "--logic" in args:
            cmd += ["-k", "TestSlug or TestDistro or TestContainer or TestProject or TestCommand or TestPkg"]
        elif "--security" in args:
            cmd += ["-k", "TestSecurity"]

        if debug:
            cmd += ["-s", "--tb=long"]

        print_section(f"MENJALANKAN TESTS (via pytest)")
        print(f"  Command: {' '.join(cmd)}\n")
    else:
        cmd = [sys.executable, test_file] + [a for a in sys.argv[1:] if not a.startswith("--")]
        print_section(f"MENJALANKAN TESTS (via unittest)")
        print(f"  Tip: Install pytest untuk output lebih baik: pip install pytest\n")

    result = subprocess.run(cmd)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()