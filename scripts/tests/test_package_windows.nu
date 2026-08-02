#!/usr/bin/env nu
# ── GitKraft · test_package_windows.nu ──────────────────────────────────────
# Tests for scripts/ci/package_windows.sh — Windows NSIS installer packaging.
#
# Regression tests for a CI failure where the fallback application icon
# (generated when packaging/windows/gitkraft.ico isn't committed) caused
# `makensis` to abort with `Error while loading icon ... can't open file`,
# even though the file existed on disk. Two earlier fixes (generating a
# 256x256, then a 32x32 icon via PowerShell + System.Drawing) both failed
# identically — the root cause was that .NET's `Icon.Save()` on an icon
# built from `Bitmap.GetHicon()` emits non-standard ICO data that NSIS's
# legacy icon loader can't parse, regardless of bitmap size.
#
# The fix replaces that runtime-generated icon with a pre-built, verified
# classic (BMP-encoded, not Vista+ PNG-compressed) 16x16 32bpp ICO, embedded
# in the script as a base64 blob and decoded with `base64 -d` — no
# PowerShell / System.Drawing dependency at all. These tests parse that
# blob directly out of the script and validate the decoded bytes form a
# well-formed classic ICO file, without needing bash or a Windows runner.

use std/assert
use runner.nu *

def script-path []: nothing -> string {
    $env.CURRENT_FILE | path dirname | path join ".." "ci" "package_windows.sh"
}

# Extract the base64 payload between the `<<'ICO_BASE64' ... ICO_BASE64` heredoc
# markers in package_windows.sh, and return it decoded as binary.
def decoded-fallback-icon-bytes []: nothing -> binary {
    let text = (open --raw (script-path))
    let lines = ($text | lines)
    let start = ($lines | enumerate | where item == "    base64 -d > \"$ICON\" <<'ICO_BASE64'" | get index.0)
    let end = ($lines | enumerate | where item == "ICO_BASE64" | get index.0)
    $lines
    | slice (($start + 1))..(($end - 1))
    | str join ""
    | decode base64
}

def "test package_windows.sh: parses cleanly as a shell script" [] {
    let result = (^bash "-n" (script-path) | complete)
    assert equal $result.exit_code 0
}

def "test package_windows.sh: embeds a base64 fallback icon heredoc" [] {
    let text = (open --raw (script-path))
    assert ($text | str contains "base64 -d > \"$ICON\" <<'ICO_BASE64'")
    assert ($text | str contains "ICO_BASE64")
}

def "test package_windows.sh: no longer runs PowerShell/System.Drawing to build the icon" [] {
    # Regression guard: this is the exact code path that produced a malformed
    # ICO that NSIS rejected, twice, at two different bitmap sizes. Prose
    # comments are allowed to mention it (to explain why it was removed);
    # the actual PowerShell invocation and .NET calls must be gone.
    let text = (open --raw (script-path))
    assert (not ($text | str contains "Add-Type -AssemblyName System.Drawing"))
    assert (not ($text | str contains "powershell -NoProfile -Command"))
}

# ── decoded ICO byte-level validation ───────────────────────────────────────

def "test fallback icon: decodes to a non-empty binary" [] {
    let bytes = (decoded-fallback-icon-bytes)
    assert (($bytes | bytes length) > 0)
}

def "test fallback icon: has the exact expected byte length" [] {
    # 6-byte ICONDIR + 16-byte ICONDIRENTRY + 40-byte BITMAPINFOHEADER +
    # 16*16*4 XOR (BGRA) bytes + 16*4 AND-mask bytes = 1150 bytes total.
    let bytes = (decoded-fallback-icon-bytes)
    assert equal ($bytes | bytes length) 1150
}

def "test fallback icon: ICONDIR header marks it as a valid single-image icon" [] {
    let bytes = (decoded-fallback-icon-bytes)
    # ICONDIR: reserved (u16) = 0, type (u16) = 1 (icon), count (u16) = 1
    assert equal ($bytes | bytes at 0..1) (0x[00 00])
    assert equal ($bytes | bytes at 2..3) (0x[01 00])
    assert equal ($bytes | bytes at 4..5) (0x[01 00])
}

def "test fallback icon: ICONDIRENTRY declares 16x16 at 32 bits per pixel" [] {
    let bytes = (decoded-fallback-icon-bytes)
    # ICONDIRENTRY starts at offset 6: width, height, colorCount, reserved
    # (each 1 byte, offsets 6-9), then planes (u16 at 10-11) and bitCount
    # (u16 at 12-13).
    assert equal ($bytes | bytes at 6..6) (0x[10])
    assert equal ($bytes | bytes at 7..7) (0x[10])
    assert equal ($bytes | bytes at 10..11) (0x[01 00])
    assert equal ($bytes | bytes at 12..13) (0x[20 00])
}

def "test fallback icon: image offset points at a classic, non-PNG BITMAPINFOHEADER" [] {
    let bytes = (decoded-fallback-icon-bytes)
    # imageOffset (u32 at bytes 12..15 of the entry, i.e. file offset 18..21)
    # must equal 22 (6 + 16), and the data at that offset must be a
    # BITMAPINFOHEADER (biSize == 40), never a PNG signature (\x89PNG).
    assert equal ($bytes | bytes at 18..21) (0x[16 00 00 00])
    assert equal ($bytes | bytes at 22..25) (0x[28 00 00 00])
    assert (not (($bytes | bytes at 22..25) == (0x[89 50 4e 47])))
}

def "test fallback icon: bytesInRes matches the actual embedded image data length" [] {
    let bytes = (decoded-fallback-icon-bytes)
    let total_len = ($bytes | bytes length)
    let image_offset = 22
    let expected_image_len = ($total_len - $image_offset)
    let declared_len = (
        $bytes
        | bytes at 14..17
        | into int --endian little
    )
    assert equal $declared_len $expected_image_len
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
