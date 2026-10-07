"""Split the verified release ZIP into transfer-sized parts (<10 GB each)."""

from __future__ import annotations

import hashlib
import shutil
from pathlib import Path


HERE = Path(__file__).resolve().parent
SOURCE = HERE / "output" / "ServerRS-Windows-离线部署包.zip"
SOURCE_HASH = SOURCE.with_suffix(".zip.sha256")
TRANSFER = HERE / "output" / "transfer"
PART_SIZE = 4_000_000_000  # decimal GB: each file stays well below a 10 GB limit
BUFFER_SIZE = 8 * 1024 * 1024


def main() -> None:
    if not SOURCE.is_file() or not SOURCE_HASH.is_file():
        raise SystemExit("Build the complete ZIP and its checksum first")
    if TRANSFER.exists() and any(TRANSFER.iterdir()):
        raise SystemExit(f"Transfer directory is not empty: {TRANSFER}")
    TRANSFER.mkdir(parents=True, exist_ok=True)

    expected = SOURCE_HASH.read_text(encoding="utf-8").split()[0].lower()
    whole = hashlib.sha256()
    manifest: list[str] = []
    index = 0

    with SOURCE.open("rb") as source:
        while True:
            first = source.read(min(BUFFER_SIZE, PART_SIZE))
            if not first:
                break
            index += 1
            part = TRANSFER / f"ServerRS-Offline.zip.{index:03d}"
            part_hash = hashlib.sha256()
            remaining = PART_SIZE
            print(f"Writing {part.name}", flush=True)
            with part.open("wb") as output:
                chunk = first
                while chunk:
                    output.write(chunk)
                    whole.update(chunk)
                    part_hash.update(chunk)
                    remaining -= len(chunk)
                    if not remaining:
                        break
                    chunk = source.read(min(BUFFER_SIZE, remaining))
            manifest.append(f"{part_hash.hexdigest()}  {part.name}")

    if whole.hexdigest() != expected:
        raise RuntimeError("Part concatenation does not match the original ZIP")

    (TRANSFER / "parts.sha256").write_text("\n".join(manifest) + "\n", encoding="utf-8")
    (TRANSFER / "archive.sha256").write_text(
        f"{expected}  ServerRS-Offline.zip\n", encoding="utf-8"
    )
    shutil.copy2(HERE / "Restore-OfflineBundle.ps1", TRANSFER / "Restore-OfflineBundle.ps1")
    (TRANSFER / "README.txt").write_text(
        "ServerRS Windows 离线部署分卷\n\n"
        "请把本目录所有文件复制到目标 Windows 主机的同一目录。\n"
        "各分卷约 4 GB，不能单独解压。目标磁盘请预留至少 40 GB 可用空间。\n"
        "在该目录打开 PowerShell，先运行 .\\Restore-OfflineBundle.ps1。\n"
        "脚本会校验分卷、合并并解压 ZIP。随后运行 .\\ServerRS\\Start-Offline.ps1。\n"
        "目标主机需安装并启动 Docker Desktop，使用 Linux 容器模式。\n"
        "完整使用说明在解压后的 ServerRS\\README.md；用户说明书在 ServerRS\\manuals。\n",
        encoding="utf-8-sig",
    )
    print(f"Completed {index} parts in {TRANSFER}", flush=True)
    print(f"Reassembled SHA-256: {expected}", flush=True)


if __name__ == "__main__":
    main()
