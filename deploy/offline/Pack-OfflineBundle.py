"""Create one Windows-friendly ZIP from a completed offline bundle.

ZIP_STORED avoids recompressing model blobs and Docker layers, and Zip64 supports
files larger than 4 GiB. Run this on the build machine after Export-OfflineBundle.ps1.
"""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
from zipfile import ZIP_STORED, ZipFile


HERE = Path(__file__).resolve().parent
BUNDLE = HERE / "bundle"
OUTPUT_DIR = HERE / "output"
OUTPUT = OUTPUT_DIR / "ServerRS-Windows-离线部署包.zip"
PARTIAL = OUTPUT.with_suffix(".zip.part")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    required = [
        BUNDLE / "images.tar",
        BUNDLE / "images.sha256",
        BUNDLE / "compose.yaml",
        BUNDLE / "Start-Offline.ps1",
        BUNDLE / "Stop-Offline.ps1",
        BUNDLE / "manuals" / "数字人系统用户文档集-后端.docx",
    ]
    for path in required:
        if not path.is_file():
            raise SystemExit(f"Missing required bundle file: {path}")

    files = sorted(p for p in BUNDLE.rglob("*") if p.is_file())
    if any(p.name.lower().startswith(".env") for p in files):
        raise SystemExit("Bundle contains a secret .env file; remove it before packaging")
    if any("-partial" in p.name for p in files):
        raise SystemExit("Bundle contains an incomplete model download")
    if PARTIAL.exists():
        raise SystemExit(f"Incomplete output exists: {PARTIAL}")

    if OUTPUT.exists():
        print(f"Validating existing archive: {OUTPUT}", flush=True)
        with ZipFile(OUTPUT, "r", allowZip64=True) as archive:
            if len(archive.infolist()) != len(files):
                raise RuntimeError("ZIP file count mismatch")
            bad = archive.testzip()
            if bad is not None:
                raise RuntimeError(f"ZIP CRC validation failed: {bad}")
        checksum = sha256(OUTPUT)
        OUTPUT.with_suffix(".zip.sha256").write_text(
            f"{checksum}  {OUTPUT.name}\n", encoding="utf-8"
        )
        print(f"Completed: {OUTPUT}", flush=True)
        print(f"SHA-256: {checksum}", flush=True)
        return

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    print(f"Packing {len(files)} files into {OUTPUT}", flush=True)
    try:
        with ZipFile(PARTIAL, "w", compression=ZIP_STORED, allowZip64=True) as archive:
            for path in files:
                name = Path("ServerRS") / path.relative_to(BUNDLE)
                print(f"  {name}", flush=True)
                archive.write(path, name.as_posix())

        with ZipFile(PARTIAL, "r", allowZip64=True) as archive:
            if len(archive.infolist()) != len(files):
                raise RuntimeError("ZIP file count mismatch")
            bad = archive.testzip()
            if bad is not None:
                raise RuntimeError(f"ZIP CRC validation failed: {bad}")

        os.replace(PARTIAL, OUTPUT)
        checksum = sha256(OUTPUT)
        OUTPUT.with_suffix(".zip.sha256").write_text(
            f"{checksum}  {OUTPUT.name}\n", encoding="utf-8"
        )
        print(f"Completed: {OUTPUT}", flush=True)
        print(f"SHA-256: {checksum}", flush=True)
    except Exception:
        # Leave the .part file for diagnosis; it is never mistaken for a release.
        raise


if __name__ == "__main__":
    main()
