"""Reject new Clippy warnings without hiding the existing diagnostic debt.

Input: cargo clippy --locked --all-targets --message-format=json > report.jsonl
Refresh the reviewed Windows baseline explicitly with --update.
"""
import argparse
from collections import Counter
import json
from pathlib import Path


def diagnostics(path):
    warnings, errors, seen = Counter(), [], set()
    completed = False
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if item.get("reason") == "build-finished":
            completed = item.get("success") is True
        if item.get("reason") != "compiler-message":
            continue
        message = item["message"]
        if message["level"] == "error":
            errors.append(message["message"])
        code = (message.get("code") or {}).get("code", "")
        if not code.startswith("clippy::") or message["level"] != "warning":
            continue
        primary = tuple((s["file_name"].replace("\\", "/"), s["line_start"], s["column_start"])
                        for s in message.get("spans", []) if s.get("is_primary"))
        identity = (code, message["message"], primary)
        if identity in seen:
            continue  # bin and bin-test emit the same source diagnostics.
        seen.add(identity)
        files = {name for name, _, _ in primary if name.startswith("src/")}
        for name in files:
            warnings[f"{name} | {code}"] += 1
    if not completed:
        errors.append("The report does not contain a successful completed build.")
    return warnings, errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--baseline", type=Path, default=Path(__file__).with_name("clippy-baseline-windows.json"))
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    warnings, errors = diagnostics(args.report)
    if errors:
        parser.exit(1, "Clippy errors:\n" + "\n".join(errors) + "\n")
    if args.update:
        args.baseline.write_text(json.dumps(dict(sorted(warnings.items())), indent=2) + "\n", encoding="utf-8")
        print(f"Recorded {sum(warnings.values())} existing warnings; this does not suppress them.")
        return
    baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
    increased = {key: (baseline.get(key, 0), count) for key, count in warnings.items() if count > baseline.get(key, 0)}
    if increased:
        parser.exit(1, "New warning budget exceeded:\n" + "\n".join(f"{key}: {old} -> {new}" for key, (old, new) in sorted(increased.items())) + "\n")
    print(f"Clippy: no new warnings ({sum(warnings.values())} existing warnings remain).")


if __name__ == "__main__":
    main()
