#!/usr/bin/env python3
"""
Builds double-clickable desktop installers:
  1. web/public/downloads/UnScanMe-Setup-0.1.0-x64.exe      (Win32 PE32+ GUI executable)
  2. web/public/downloads/UnScanMe-0.1.0-universal.dmg      (UDIF macOS disk image)
  3. web/public/downloads/UnScanMe-Installer-0.1.0-linux-x86_64 (Linux ELF GUI installer)
"""

from __future__ import annotations

import argparse
import base64
import datetime
import io
import os
import pathlib
import shutil
import stat
import struct
import subprocess
import tarfile
import tempfile
import uuid
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
INSTALLERS_DIR = ROOT / "installers"
OUT_DIR = ROOT / "web" / "public" / "downloads"
ICON_PATH = ROOT / "desktop" / "assets" / "favicon.ico"
LINUX_SVG_PATH = ROOT / "pkgbuild" / "unscanme.svg"
LINUX_DESKTOP_PATH = ROOT / "pkgbuild" / "unscanme.desktop"
LINUX_RELEASE_APP = ROOT / "desktop" / "target" / "dx" / "unscanme" / "release" / "linux" / "app"

WINDOWS_EXE_NAME = "UnScanMe-Setup-0.1.0-x64.exe"
MACOS_DMG_NAME = "UnScanMe-0.1.0-universal.dmg"
LINUX_BIN_NAME = "UnScanMe-Installer-0.1.0-linux-x86_64"

DEF_FILES = {
    "kernel32.def": """LIBRARY kernel32.dll
EXPORTS
GetProcessHeap
HeapAlloc
HeapFree
GetTempPathW
CreateFileW
WriteFile
CloseHandle
DeleteFileW
WaitForSingleObject
GetExitCodeProcess
ExitProcess
""",
    "user32.def": """LIBRARY user32.dll
EXPORTS
MessageBoxW
""",
    "shell32.def": """LIBRARY shell32.dll
EXPORTS
ShellExecuteExW
""",
}

MANIFEST_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="0.1.0.0" processorArchitecture="amd64" name="UnScanMe.Setup" type="win32"/>
  <description>UnScanMe Local Document Workbench Installer</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
    </application>
  </compatibility>
</assembly>
"""

MACOS_INFO_PLIST = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>UnScanMe</string>
  <key>CFBundleExecutable</key>
  <string>unscanme</string>
  <key>CFBundleIconFile</key>
  <string>favicon.ico</string>
  <key>CFBundleIdentifier</key>
  <string>com.unscanme.app</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>UnScanMe</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
"""

MACOS_LAUNCHER_SH = """#!/bin/bash
set -euo pipefail

RELEASE_API="https://api.github.com/repos/J0seph-Mart1n/ImgToPdf-Rust/releases/latest"
SELF_BUNDLE="$(cd "$(dirname "$0")/../.." && pwd)"

if [[ "$SELF_BUNDLE" == /Volumes/* ]]; then
  osascript -e 'display dialog "Install UnScanMe to /Applications?" buttons {"Cancel", "Install"} default button "Install" with title "UnScanMe Installer"' >/dev/null
  rm -rf "/Applications/UnScanMe.app"
  cp -R "$SELF_BUNDLE" "/Applications/UnScanMe.app"
  exec "/Applications/UnScanMe.app/Contents/MacOS/unscanme" "$@"
fi

DMG_URL="$(curl -fsSL "$RELEASE_API" 2>/dev/null | grep -Eo 'https://[^"]+\\.dmg' | head -n1 || true)"
if [[ -n "$DMG_URL" ]]; then
  TMP_DMG="${TMPDIR:-/tmp}/UnScanMeRelease.dmg"
  curl -fL "$DMG_URL" -o "$TMP_DMG"
  MNT_DIR="$(hdiutil attach "$TMP_DMG" -nobrowse -readonly | awk '/\\/Volumes\\//{for(i=3;i<=NF;i++) printf "%s%s", $i, (i<NF?" ":""); exit}')"
  if [[ -n "$MNT_DIR" && -d "$MNT_DIR/UnScanMe.app" ]]; then
    cp -R "$MNT_DIR/UnScanMe.app" "/Applications/UnScanMe.app.new"
    hdiutil detach "$MNT_DIR" -quiet || true
    rm -rf "/Applications/UnScanMe.app"
    mv "/Applications/UnScanMe.app.new" "/Applications/UnScanMe.app"
    exec open "/Applications/UnScanMe.app"
  fi
fi

osascript -e 'display dialog "Could not download UnScanMe for macOS. Please check your internet connection and try again." buttons {"OK"} default button "OK" with icon caution with title "UnScanMe Installer"'
exit 1
"""


def find_lld_link() -> str:
    rustup_lld = (
        pathlib.Path.home()
        / ".rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/lld-link"
    )
    if rustup_lld.is_file():
        return str(rustup_lld)
    on_path = shutil.which("lld-link")
    if on_path:
        return on_path
    raise FileNotFoundError(f"lld-link not found at {rustup_lld} or in PATH")


def build_windows_exe(out_path: pathlib.Path) -> None:
    lld_link = find_lld_link()
    with tempfile.TemporaryDirectory() as tmp:
        td = pathlib.Path(tmp)
        libs: list[str] = []
        for name, body in DEF_FILES.items():
            def_p = td / name
            lib_p = td / name.replace(".def", ".lib")
            def_p.write_text(body, encoding="utf-8")
            subprocess.run(
                ["llvm-dlltool", "-m", "i386:x86-64", "-d", str(def_p), "-l", str(lib_p)],
                check=True,
            )
            libs.append(str(lib_p))

        (td / "setup.manifest").write_text(MANIFEST_XML, encoding="utf-8")
        rc_p = td / "setup.rc"
        rc_p.write_text(
            f'1 ICON "{ICON_PATH.as_posix()}"\n1 24 "setup.manifest"\n',
            encoding="utf-8",
        )
        res_p = td / "setup.res"
        subprocess.run(["llvm-rc", f"/fo{res_p}", str(rc_p)], cwd=td, check=True)

        obj_p = td / "windows_setup.obj"
        subprocess.run(
            [
                "clang",
                "--target=x86_64-pc-windows-msvc",
                "-ffreestanding",
                "-fno-builtin",
                "-fno-stack-protector",
                "-mno-stack-arg-probe",
                "-Os",
                "-c",
                str(INSTALLERS_DIR / "windows_setup.c"),
                "-o",
                str(obj_p),
            ],
            check=True,
        )

        subprocess.run(
            [
                lld_link,
                "/subsystem:windows",
                "/entry:WinMainCRTStartup",
                "/nodefaultlib",
                f"/out:{out_path}",
                str(obj_p),
                str(res_p),
                *libs,
            ],
            check=True,
        )


# --- Minimal Spec-Compliant ISO-9660 + RockRidge + Apple UDIF (.dmg) Builder ---

SECTOR = 2048


def _both16(v: int) -> bytes:
    return struct.pack("<H", v) + struct.pack(">H", v)


def _both32(v: int) -> bytes:
    return struct.pack("<I", v) + struct.pack(">I", v)


def _iso_date7() -> bytes:
    now = datetime.datetime.now(datetime.timezone.utc)
    yr = max(0, min(255, now.year - 1900))
    return bytes([yr, now.month, now.day, now.hour, now.minute, min(59, now.second), 0])


def _rr_px(mode: int, is_dir: bool) -> bytes:
    # RockRidge PX entry (44 bytes: sig 'PX', len 44, ver 1, mode, nlink, uid, gid, serial)
    st_mode = (0o040000 if is_dir else 0o100000) | mode
    nlink = 2 if is_dir else 1
    return b"PX" + bytes([44, 1]) + _both32(st_mode) + _both32(nlink) + _both32(0) + _both32(0) + _both32(1)


def _rr_nm(name: str) -> bytes:
    raw = name.encode("utf-8")
    return b"NM" + bytes([5 + len(raw), 1, 0]) + raw


def _dir_record(
    iso_name: bytes,
    extent_lba: int,
    size: int,
    is_dir: bool,
    rr_extra: bytes = b"",
    is_root: bool = False,
) -> bytes:
    flags = 0x02 if is_dir else 0x00
    name_len = len(iso_name)
    pad = 1 if (name_len % 2 == 0) else 0
    rec = bytearray(33 + name_len + pad + len(rr_extra))
    rec[0] = len(rec)
    rec[1] = 0
    rec[2:10] = _both32(extent_lba)
    rec[10:18] = _both32(size)
    rec[18:25] = _iso_date7()
    rec[25] = flags
    rec[26] = 0
    rec[27] = 0
    rec[28:32] = _both16(1)
    rec[32] = name_len
    rec[33 : 33 + name_len] = iso_name
    pos = 33 + name_len + pad
    rec[pos : pos + len(rr_extra)] = rr_extra
    if len(rec) % 2 == 1:
        rec.append(0)
        rec[0] = len(rec)
    return bytes(rec)


def _pack_dir_entries(entries: list[bytes]) -> bytes:
    out = bytearray()
    cur_sec = bytearray()
    for e in entries:
        if len(cur_sec) + len(e) > SECTOR:
            cur_sec.extend(b"\x00" * (SECTOR - len(cur_sec)))
            out.extend(cur_sec)
            cur_sec = bytearray()
        cur_sec.extend(e)
    if cur_sec or not out:
        cur_sec.extend(b"\x00" * (SECTOR - len(cur_sec)))
        out.extend(cur_sec)
    return bytes(out)


class _Node:
    def __init__(self, name: str, is_dir: bool, data: bytes = b"", mode: int = 0o755):
        self.name = name
        self.is_dir = is_dir
        self.data = data
        self.mode = mode
        self.children: list[_Node] = []
        self.lba = 0
        self.size = len(data)
        self.parent: _Node | None = None


def build_iso9660_rockridge(root_node: _Node, volume_id: str) -> bytes:
    nodes_bfs: list[_Node] = []
    dir_nodes: list[_Node] = []
    file_nodes: list[_Node] = []

    def walk(n: _Node) -> None:
        nodes_bfs.append(n)
        if n.is_dir:
            dir_nodes.append(n)
            for c in n.children:
                c.parent = n
                walk(c)
        else:
            file_nodes.append(n)

    root_node.parent = root_node
    walk(root_node)

    # Reserve LBAs: 0..15 system, 16 PVD, 17 TermVD, 18.. directory extents, then file extents
    next_lba = 18

    # Estimate each directory size by building placeholder records
    for d in dir_nodes:
        self_extra = (
            b"SP\x07\x01\xbe\xef\x00"
            + _rr_px(0o755, True)
            + b"ER"
            + bytes([18, 1, 10, 0, 0, 1])
            + b"RRIP_1991A"
            if d is root_node
            else _rr_px(0o755, True)
        )
        dummy_entries = [
            _dir_record(b"\x00", 0, SECTOR, True, self_extra, is_root=(d is root_node)),
            _dir_record(b"\x01", 0, SECTOR, True, _rr_px(0o755, True)),
        ]
        for idx, c in enumerate(d.children):
            short = f"F{idx}".encode("ascii") if not c.is_dir else f"D{idx}".encode("ascii")
            iso_nm = short if c.is_dir else short + b";1"
            dummy_entries.append(_dir_record(iso_nm, 0, c.size, c.is_dir, _rr_px(c.mode, c.is_dir) + _rr_nm(c.name)))
        packed = _pack_dir_entries(dummy_entries)
        d.lba = next_lba
        d.size = len(packed)
        next_lba += len(packed) // SECTOR

    for f in file_nodes:
        f.lba = next_lba
        sectors = max(1, (f.size + SECTOR - 1) // SECTOR)
        next_lba += sectors

    total_sectors = next_lba + 2

    # Emit actual directory extents with resolved LBAs
    dir_bytes: list[bytes] = []
    for d in dir_nodes:
        entries = []
        self_extra = (
            b"SP\x07\x01\xbe\xef\x00"
            + _rr_px(0o755, True)
            + b"ER"
            + bytes([18, 1, 10, 0, 0, 1])
            + b"RRIP_1991A"
            if d is root_node
            else _rr_px(0o755, True)
        )
        entries.append(_dir_record(b"\x00", d.lba, d.size, True, self_extra, is_root=(d is root_node)))
        entries.append(_dir_record(b"\x01", d.parent.lba, d.parent.size, True, _rr_px(0o755, True)))
        for idx, c in enumerate(d.children):
            short = f"F{idx}".encode("ascii") if not c.is_dir else f"D{idx}".encode("ascii")
            iso_nm = short if c.is_dir else short + b";1"
            entries.append(_dir_record(iso_nm, c.lba, c.size, c.is_dir, _rr_px(c.mode, c.is_dir) + _rr_nm(c.name)))
        dir_bytes.append(_pack_dir_entries(entries))

    # Build Primary Volume Descriptor (sector 16)
    pvd = bytearray(SECTOR)
    pvd[0] = 1
    pvd[1:6] = b"CD001"
    pvd[6] = 1
    pvd[8:40] = b"MACOSX".ljust(32, b" ")
    pvd[40:72] = volume_id.encode("ascii")[:32].ljust(32, b" ")
    pvd[80:88] = _both32(total_sectors)
    pvd[120:124] = _both16(1)
    pvd[124:128] = _both16(1)
    pvd[128:132] = _both16(SECTOR)
    pvd[132:140] = _both32(0)  # path table unused by RockRidge/ISO readers on macOS/libarchive
    root_rec = _dir_record(b"\x00", root_node.lba, root_node.size, True, b"", is_root=True)
    pvd[156 : 156 + len(root_rec)] = root_rec
    pvd[881] = 1

    tvd = bytearray(SECTOR)
    tvd[0] = 255
    tvd[1:6] = b"CD001"
    tvd[6] = 1

    img = bytearray(total_sectors * SECTOR)
    img[16 * SECTOR : 17 * SECTOR] = pvd
    img[17 * SECTOR : 18 * SECTOR] = tvd

    for d, b in zip(dir_nodes, dir_bytes):
        img[d.lba * SECTOR : d.lba * SECTOR + len(b)] = b

    for f in file_nodes:
        img[f.lba * SECTOR : f.lba * SECTOR + f.size] = f.data

    return bytes(img)


def wrap_udif_dmg(raw_iso: bytes, out_path: pathlib.Path) -> None:
    sectors_512 = len(raw_iso) // 512
    compressed = zlib.compress(raw_iso, level=6)
    crc = zlib.crc32(raw_iso) & 0xFFFFFFFF

    checksum_words = struct.pack(">I", crc) + (b"\x00" * (128 - 4))
    no_checksum_words = b"\x00" * 128
    chunk_data = struct.pack(">IIQQQQ", 0x80000005, 0, 0, sectors_512, 0, len(compressed))
    chunk_term = struct.pack(">IIQQQQ", 0xFFFFFFFF, 0, sectors_512, 0, len(compressed), 0)

    mish = (
        struct.pack(">4sIQQQII", b"mish", 1, 0, sectors_512, 0, 1024, 0)
        + (b"\x00" * 24)
        + struct.pack(">II", 2, 32)
        + checksum_words
        + struct.pack(">I", 2)
        + chunk_data
        + chunk_term
    )
    mish_b64 = base64.b64encode(mish).decode("ascii")

    plist_xml = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
        '<plist version="1.0">\n'
        "<dict>\n"
        "\t<key>resource-fork</key>\n"
        "\t<dict>\n"
        "\t\t<key>blkx</key>\n"
        "\t\t<array>\n"
        "\t\t\t<dict>\n"
        "\t\t\t\t<key>Attributes</key>\n"
        "\t\t\t\t<string>0x0050</string>\n"
        "\t\t\t\t<key>CFName</key>\n"
        "\t\t\t\t<string>whole disk (Apple_ISO : 0)</string>\n"
        "\t\t\t\t<key>Data</key>\n"
        f"\t\t\t\t<data>{mish_b64}</data>\n"
        "\t\t\t\t<key>ID</key>\n"
        "\t\t\t\t<string>0</string>\n"
        "\t\t\t\t<key>Name</key>\n"
        "\t\t\t\t<string>whole disk (Apple_ISO : 0)</string>\n"
        "\t\t\t</dict>\n"
        "\t\t</array>\n"
        "\t</dict>\n"
        "</dict>\n"
        "</plist>\n"
    ).encode("utf-8")

    xml_offset = len(compressed)
    koly = (
        struct.pack(">4sIIIQQQQQII", b"koly", 4, 512, 1, 0, 0, xml_offset, 0, 0, 1, 1)
        + uuid.uuid4().bytes
        + struct.pack(">II", 0, 0)
        + no_checksum_words
        + struct.pack(">QQ", xml_offset, len(plist_xml))
        + (b"\x00" * 120)
        + struct.pack(">II", 0, 0)
        + no_checksum_words
        + struct.pack(">IQ", 1, sectors_512)
        + (b"\x00" * 12)
    )
    assert len(koly) == 512

    out_path.write_bytes(compressed + plist_xml + koly)


def build_macos_dmg(out_path: pathlib.Path) -> pathlib.Path:
    root = _Node("", True)
    app = _Node("UnScanMe.app", True, mode=0o755)
    contents = _Node("Contents", True, mode=0o755)
    macos_dir = _Node("MacOS", True, mode=0o755)
    res_dir = _Node("Resources", True, mode=0o755)

    macos_dir.children.append(_Node("unscanme", False, MACOS_LAUNCHER_SH.encode("utf-8"), mode=0o755))
    res_dir.children.append(_Node("favicon.ico", False, ICON_PATH.read_bytes(), mode=0o644))
    contents.children.extend(
        [
            _Node("Info.plist", False, MACOS_INFO_PLIST.encode("utf-8"), mode=0o644),
            _Node("PkgInfo", False, b"APPL????", mode=0o644),
            macos_dir,
            res_dir,
        ]
    )
    app.children.append(contents)
    root.children.append(app)

    iso_bytes = build_iso9660_rockridge(root, "UnScanMe")
    iso_check_path = INSTALLERS_DIR / "_verify_macos.iso"
    iso_check_path.write_bytes(iso_bytes)
    wrap_udif_dmg(iso_bytes, out_path)
    return iso_check_path


def build_linux_elf_installer(out_path: pathlib.Path, skip_build: bool = False) -> None:
    bin_src = LINUX_RELEASE_APP / "unscanme"
    if not skip_build or not bin_src.is_file():
        env = os.environ.copy()
        env.setdefault("CARGO_NET_OFFLINE", "true")
        subprocess.run(["dx", "build", "--desktop", "--release"], cwd=ROOT / "desktop", env=env, check=True)

    with tempfile.TemporaryDirectory() as tmp:
        td = pathlib.Path(tmp)
        stripped_bin = td / "unscanme"
        shutil.copy2(bin_src, stripped_bin)
        if shutil.which("strip"):
            subprocess.run(["strip", "--strip-unneeded", str(stripped_bin)], check=False)

        payload_tar = td / "payload.tar.gz"
        with tarfile.open(payload_tar, mode="w:gz", compresslevel=9) as tf:
            tf.add(stripped_bin, arcname="unscanme")
            assets_dir = LINUX_RELEASE_APP / "assets"
            if assets_dir.is_dir():
                for item in sorted(assets_dir.rglob("*")):
                    if item.is_file():
                        tf.add(item, arcname=f"assets/{item.relative_to(assets_dir).as_posix()}")
            tf.add(LINUX_SVG_PATH, arcname="unscanme.svg")
            tf.add(LINUX_DESKTOP_PATH, arcname="unscanme.desktop")

        payload_s = td / "payload.s"
        payload_s.write_text(
            '.section .rodata\n'
            '.globl _unscanme_payload_start\n'
            '.globl _unscanme_payload_end\n'
            '_unscanme_payload_start:\n'
            f'.incbin "{payload_tar.as_posix()}"\n'
            '_unscanme_payload_end:\n'
            '.section .note.GNU-stack,"",@progbits\n',
            encoding="utf-8",
        )

        subprocess.run(
            [
                "clang",
                "-Os",
                "-Wall",
                "-Wextra",
                str(INSTALLERS_DIR / "linux_installer.c"),
                str(payload_s),
                "-o",
                str(out_path),
            ],
            check=True,
        )
    out_path.chmod(out_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--linux-only", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for obsolete in (
        "i-hate-scans-Setup-0.1.0-x64.exe",
        "i-hate-scans-0.1.0-universal.dmg",
        "i-hate-scans-Installer-0.1.0-linux-x86_64",
        "DocForge-Setup-0.1.0-x64.exe",
        "DocForge-0.1.0-universal.dmg",
        "DocForge-Installer-0.1.0-linux-x86_64",
        "DocForge-Setup-Windows.ps1",
        "DocForge-Installer-macOS.command",
        "DocForge-0.1.0-linux-x86_64.run",
        "install-linux.sh",
        "img-to-pdf_0.1.0_amd64.deb",
        "img_to_pdf-0.1.0-1.x86_64.rpm",
        "img-to-pdf-0.1.0-1-x86_64.pkg.tar.zst",
        "PKGBUILD",
    ):
        p = OUT_DIR / obsolete
        if p.exists():
            p.unlink()

    linux_path = OUT_DIR / LINUX_BIN_NAME
    build_linux_elf_installer(linux_path, skip_build=args.skip_build)
    print("Linux universal installer:", linux_path, linux_path.stat().st_size, "bytes")

    if not args.linux_only:
        exe_path = OUT_DIR / WINDOWS_EXE_NAME
        dmg_path = OUT_DIR / MACOS_DMG_NAME
        build_windows_exe(exe_path)
        iso_check_path = build_macos_dmg(dmg_path)
        print("Windows installer:", exe_path, exe_path.stat().st_size, "bytes")
        print("macOS DMG:", dmg_path, dmg_path.stat().st_size, "bytes")
        subprocess.run(["file", str(exe_path), str(dmg_path), str(linux_path)], check=True)
        subprocess.run(["bsdtar", "-tvf", str(iso_check_path)], check=True)
        iso_check_path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
