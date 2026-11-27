"""Fail-closed ClamAV gate for macOS/Linux packages; definitions stay out of payloads."""
from pathlib import Path
import datetime as dt
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def require_success(code, phase):
    if code != 0:
        raise RuntimeError(f"{phase} failed or detected a threat (exit {code}); packages are blocked")


# Normally require builds within 48 h. During a publisher pause, accept the
# current official daily version only with evidence from this successful
# FreshClam invocation. Never accept an unbounded/offline stale database.
RECENT_DATABASE_HOURS = 48
CURRENT_DATABASE_MAX_HOURS = 7 * 24


def current_daily_confirmed(version, update_log):
    fields = version.strip().split("/", 2)
    if len(fields) != 3 or not fields[1].isdigit():
        return False
    daily = re.findall(
        r"\bdaily\.(?:cvd|cld) (?:database is up-to-date|updated) "
        r"\(version: (\d+),", update_log
    )
    return bool(daily) and daily[-1] == fields[1]


def database_timestamp(version, now, update_log=""):
    fields = version.strip().split("/", 2)
    if len(fields) != 3 or not fields[1].isdigit():
        raise RuntimeError("Scanner did not report its database version/date")
    try:
        updated = dt.datetime.strptime(fields[2].strip(), "%a %b %d %H:%M:%S %Y").replace(tzinfo=dt.timezone.utc)
    except ValueError as error:
        raise RuntimeError("Scanner reported an invalid database timestamp: " + version) from error
    age = (now - updated).total_seconds() / 3600
    if -24 <= age <= RECENT_DATABASE_HOURS:
        return updated
    if (RECENT_DATABASE_HOURS < age <= CURRENT_DATABASE_MAX_HOURS
            and current_daily_confirmed(version, update_log)):
        return updated
    raise RuntimeError(
        f"ClamAV database {fields[1]} built {updated.isoformat()} is {age:.1f} hours old "
        f"at {now.isoformat()}. Require a build within {RECENT_DATABASE_HOURS} hours, "
        f"or the exact current daily version confirmed by this FreshClam update "
        f"within {CURRENT_DATABASE_MAX_HOURS} hours. Packages are blocked."
    )


def inventory(directory):
    result = []
    for path in sorted(directory.iterdir()):
        if path.name.endswith((".zip", ".dmg", ".AppImage", ".deb", ".tar.gz")):
            with path.open("rb") as source:
                result.append({"name": path.name, "sha256": hashlib.file_digest(source, "sha256").hexdigest()})
    if not result:
        raise RuntimeError("No native distribution packages to scan")
    lines = (directory / "SHA256SUMS.txt").read_text().splitlines()
    for item in result:
        if f"{item['sha256']}  {item['name']}" not in lines:
            raise RuntimeError("Native package checksum manifest mismatch: " + item["name"])
    return result


def scan(root=ROOT):
    reports = root / "work/package-security/native"
    reports.mkdir(parents=True, exist_ok=True)
    report = {"status": "failed", "commit": os.environ.get("GITHUB_SHA", "local"), "engine": "ClamAV", "error": None}
    env = dict(os.environ, LC_ALL="C", TZ="UTC")

    def command(args, name):
        with (reports / name).open("w", encoding="utf-8") as log:
            result = subprocess.run(list(map(str, args)), stdout=log, stderr=subprocess.STDOUT, env=env, timeout=900)
        output = (reports / name).read_text(encoding="utf-8", errors="replace")
        if result.returncode != 0:
            # Surface the actual updater/scanner diagnostic in Actions, not only
            # in an artifact. Updater failures are not malware detections.
            print(f"--- {name} (exit {result.returncode}) ---", file=sys.stderr)
            print(output[-16000:], file=sys.stderr)
            phase = "definition update" if name == "intelligence-update.log" else "antivirus command"
            report["failedCommand"] = {"phase": phase, "log": name, "exitCode": result.returncode}
            raise RuntimeError(f"ClamAV {phase} failed (exit {result.returncode}); packages are blocked. See {name} above and in the security-report artifact.")
        return output

    try:
        scanner, updater = shutil.which("clamscan"), shutil.which("freshclam")
        if not scanner or not updater:
            raise RuntimeError("ClamAV scanner/updater unavailable; no scan performed")
        directory = root / "dist"
        report["files"] = inventory(directory)
        mode = os.environ.get("LIBRETOON_CLAMAV_MODE", "isolated")
        if mode == "ubuntu-system":
            # Ubuntu's packaged updater uses distro-managed paths (and may be
            # confined by AppArmor). Never place its configuration in RUNNER_TEMP.
            # Only the updater needs sudo; scanning the packages stays unprivileged.
            if not sys.platform.startswith("linux"):
                raise RuntimeError("ubuntu-system ClamAV mode requires Linux")
            database = Path("/var/lib/clamav")
            config = Path("/etc/clamav/freshclam.conf")
            if not config.is_file() or not database.is_dir():
                raise RuntimeError("Ubuntu ClamAV configuration/database directory is missing")
            update_args = ["sudo", "-n", updater, f"--config-file={config}",
                           f"--datadir={database}", "--stdout"]
        elif mode == "isolated":
            database = Path(os.environ.get("RUNNER_TEMP", root / "work")) / "libretoon-clamav"
            database.mkdir(parents=True, exist_ok=True)
            config = database / "freshclam.conf"
            config.write_text(f'DatabaseDirectory "{database}"\nDatabaseMirror database.clamav.net\nForeground yes\nTestDatabases yes\n', encoding="utf-8")
            update_args = [updater, f"--config-file={config}", "--stdout"]
        else:
            raise RuntimeError("Unknown ClamAV mode: " + mode)
        report.update(databaseMode=mode, databaseDirectory=str(database), updaterConfig=str(config))
        update_log = command(update_args, "intelligence-update.log")
        version = command([scanner, f"--database={database}", "--version"], "scanner-version.log").strip()
        now = dt.datetime.now(dt.timezone.utc)
        report.update(scannerVersion=version, timeUtc=now.isoformat(),
                      intelligenceCheckedUtc=now.isoformat(),
                      currentDailyConfirmed=current_daily_confirmed(version, update_log))
        updated = database_timestamp(version, now, update_log)
        age = (now - updated).total_seconds() / 3600
        report.update(intelligenceUpdatedUtc=updated.isoformat(), intelligenceAgeHours=round(age, 2),
                      freshnessPolicy="recent-build" if age <= RECENT_DATABASE_HOURS else "freshclam-current-daily-version")
        print(f"ClamAV: {version}; database age {age:.1f} hours; "
              f"freshness policy: {report['freshnessPolicy']}")
        # Archive scanning is explicit; embedded executables and final containers are checked.
        payloads = [path for path in (root / "work/packages/macos/LibreToon.app", root / "work/packages/linux/LibreToon.AppDir") if path.is_dir()]
        if not payloads:
            raise RuntimeError("Unpacked native payload missing; cannot verify embedded application resources")
        command([scanner, f"--database={database}", "--recursive", "--scan-archive=yes", "--alert-exceeds-max=yes", "--max-filesize=512M", "--max-scansize=2048M", "--infected", directory, *payloads], "scan.log")
        if report["files"] != inventory(directory):
            raise RuntimeError("Packages changed during antivirus analysis")
        report["status"] = "passed"
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        (reports / "security-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        summary = f"Native distribution checks: {report['status']}. See security-report.json for engine, definitions and hashes. A passing scan does not guarantee absence of malware or removal of browser warnings.\n"
        (reports / "README.txt").write_text(summary, encoding="utf-8")
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as output:
                output.write(summary)


if __name__ == "__main__":
    try:
        scan()
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
