#!/usr/bin/env python3
"""Packs a built wallet and the documents of the repository in a release archive.

    python3 tools/pack.py VERSION PLATFORM PROGRAM OUT_DIR EPOCH

PLATFORM is windows-x86_64 (a .zip), or linux-x86_64, macos-aarch64 or macos-x86_64 (a .tar.gz). The archive holds
one folder,
neb-wallet-desktop-VERSION-PLATFORM, with the program, README.md, CHANGELOG.md and LICENSE. Every entry carries the
time EPOCH (seconds, UTC) and fixed owners and modes, so the same files give the same archive. Prints the SHA-256.
"""
import gzip
import hashlib
import io
import os
import sys
import tarfile
import time
import zipfile

DOCUMENTS = ("README.md", "CHANGELOG.md", "LICENSE")


def main() -> None:
    if len(sys.argv) != 6:
        sys.exit(__doc__)
    version, platform, program, out_dir, epoch = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
    if platform not in ("windows-x86_64", "linux-x86_64", "macos-aarch64", "macos-x86_64"):
        sys.exit("unknown platform: " + platform)
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    folder = "neb-wallet-desktop-%s-%s" % (version, platform)
    entries = [(os.path.basename(program), program, 0o755)] + [(name, os.path.join(root, name), 0o644) for name in DOCUMENTS]
    os.makedirs(out_dir, exist_ok=True)
    if platform.startswith("windows"):
        path = os.path.join(out_dir, folder + ".zip")
        stamp = time.gmtime(epoch)[:6]
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for name, source, mode in entries:
                info = zipfile.ZipInfo(folder + "/" + name, stamp)
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = mode << 16
                with open(source, "rb") as f:
                    archive.writestr(info, f.read())
    else:
        path = os.path.join(out_dir, folder + ".tar.gz")
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            for name, source, mode in entries:
                with open(source, "rb") as f:
                    data = f.read()
                info = tarfile.TarInfo(folder + "/" + name)
                info.size, info.mtime, info.mode = len(data), epoch, mode
                info.uid = info.gid = 0
                info.uname = info.gname = "root"
                archive.addfile(info, io.BytesIO(data))
        with open(path, "wb") as out, gzip.GzipFile(filename="", fileobj=out, mode="wb", compresslevel=9, mtime=epoch) as z:
            z.write(raw.getvalue())
    with open(path, "rb") as f:
        print(hashlib.sha256(f.read()).hexdigest() + "  " + os.path.basename(path))


if __name__ == "__main__":
    main()
