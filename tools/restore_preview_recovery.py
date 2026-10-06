"""Verify or restore the preserved preview evidence; never overwrite changed files."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import zipfile


def digest(handle):
    result = hashlib.sha256()
    for block in iter(lambda: handle.read(1024 * 1024), b""):
        result.update(block)
    return result.hexdigest()


def file_digest(path):
    with path.open("rb") as handle:
        return digest(handle)


def destination_for(member, roots):
    path = PurePosixPath(member)
    parts = path.parts
    if len(parts) < 2 or parts[0] not in roots or path.is_absolute() or ".." in parts or "\\" in member or ":" in member:
        raise ValueError("invalid recovery path: " + member)
    root = roots[parts[0]].resolve()
    destination = root.joinpath(*parts[1:]).resolve()
    destination.relative_to(root)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    checkout = Path(__file__).resolve().parents[1]
    parser.add_argument("--manifest", type=Path, default=checkout / "docs/conformance/2026-08-support/recovery/manifest.json")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--sysml-root", type=Path, default=checkout)
    parser.add_argument("--stack-root", type=Path, default=checkout.parent)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    if manifest["schema"] != "dev.mercurio.preview-recovery.v1" or manifest["candidate_promoted"] or manifest["full_release_qualified"]:
        raise ValueError("invalid recovery scope")
    roots = {"sysml": args.sysml_root, "stack": args.stack_root}
    rows = manifest["files"]
    if len({row["member"] for row in rows}) != len(rows):
        raise ValueError("duplicate recovery member")
    groups = {}
    for row in rows:
        destination_for(row["member"], roots)
        groups.setdefault(row["archive"], []).append(row)
    archives = {row["path"]: row for row in manifest["archives"]}
    if len(archives) != len(manifest["archives"]) or set(groups) != set(archives):
        raise ValueError("recovery archive inventory mismatch")
    # Check every archive, member and pre-existing destination before writing.
    for name, archive_row in archives.items():
        if Path(name).name != name or "\\" in name or ":" in name:
            raise ValueError("invalid archive path")
        path = args.manifest.parent / name
        if path.stat().st_size != archive_row["bytes"] or file_digest(path) != archive_row["sha256"]:
            raise ValueError("archive integrity mismatch: " + name)
        with zipfile.ZipFile(path) as archive:
            if len(archive.namelist()) != len(groups[name]) or set(archive.namelist()) != {row["member"] for row in groups[name]}:
                raise ValueError("archive member inventory mismatch")
            for row in groups[name]:
                info = archive.getinfo(row["member"])
                with archive.open(info) as handle:
                    if info.file_size != row["bytes"] or digest(handle) != row["sha256"]:
                        raise ValueError("member integrity mismatch: " + row["member"])
                destination = destination_for(row["member"], roots)
                if not args.verify_only and destination.exists() and file_digest(destination) != row["sha256"]:
                    raise ValueError("refusing to overwrite changed evidence: " + str(destination))
    restored = 0
    if not args.verify_only:
        for name in archives:
            with zipfile.ZipFile(args.manifest.parent / name) as archive:
                for row in groups[name]:
                    destination = destination_for(row["member"], roots)
                    if destination.exists():
                        continue
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    # Exclusive creation prevents a concurrent file from being replaced.
                    with archive.open(row["member"]) as source, destination.open("xb") as output:
                        for block in iter(lambda: source.read(1024 * 1024), b""):
                            output.write(block)
                    if file_digest(destination) != row["sha256"]:
                        raise ValueError("restored file integrity mismatch")
                    restored += 1
    print(json.dumps({"verified_files": len(rows), "archives": len(archives), "restored_files": restored,
                      "candidate_promoted": False, "full_release_qualified": False}))


if __name__ == "__main__":
    main()
