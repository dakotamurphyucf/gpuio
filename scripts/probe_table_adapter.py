#!/usr/bin/env python3
"""Create an isolated OCH-39 candidate workspace; never change project pins.

Supply a local checkout of the exact upstream revision. Output must be a new
directory beneath this repository's ignored scratch/. The caller runs Cargo
through scripts/gpuio so the project toolchain remains isolated.
"""

import argparse
import io
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

KIT_REV = "84f57fdfcb4910623fb0bb7f795b077e249f9271"
GPUI_REV = "a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b"


def replace_once(text, pattern, replacement):
    result, count = re.subn(pattern, lambda _: replacement, text, count=1, flags=re.M | re.S)
    if count != 1:
        raise ValueError(f"upstream manifest no longer matches: {pattern}")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--macro-fallback", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    source = args.source.resolve()
    output = args.output.resolve()
    if not output.is_relative_to(root / "scratch") or output.exists():
        parser.error("output must be a new directory beneath this checkout's scratch/")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    if revision != KIT_REV:
        parser.error(f"source HEAD must be {KIT_REV}; found {revision}")
    # Archive committed input; never edit or copy local working-tree changes.
    archive_bytes = subprocess.check_output(["git", "archive", KIT_REV], cwd=source)
    output.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive_bytes)) as archive:
        archive.extractall(output, filter="data")
    manifest = output / "Cargo.toml"
    text = replace_once(
        manifest.read_text(),
        r'default-members = .*?resolver = "2"',
        'default-members = ["crates/component"]\n'
        'members = ["crates/component", "crates/component-macros", "crates/assets"]\n'
        'resolver = "3"',
    )
    text = replace_once(text, r'^gpui-base = [^\n]*', f'gpui-base = {{ path = "{root / "vendor/gpui-base"}" }}')
    for alias, package in (
        ("gpui", "gpui"),
        ("gpui_platform", "gpui_platform"),
        ("gpui_macros", "gpui_macros"),
        ("reqwest_client", "reqwest_client"),
        ("sum-tree", "sum_tree"),
    ):
        features = ", default-features = false" if alias == "gpui" else ""
        text = replace_once(
            text,
            rf"^{re.escape(alias)} = [^\n]*",
            f'{alias} = {{ package = "{package}", git = "https://github.com/zed-industries/zed.git", '
            f'rev = "{GPUI_REV}"{features} }}',
        )
    text = replace_once(
        text,
        r'\[profile.dev\].*?\[workspace.metadata.typos\]',
        '[profile.dev]\ndebug = 0\n\n[profile.dev.package."*"]\nopt-level = 1\n\n'
        '[patch.crates-io]\n'
        f'accesskit_macos = {{ path = "{root / "vendor/accesskit-macos"}" }}\n\n'
        '[patch."https://github.com/zed-industries/zed.git"]\n'
        f'gpui = {{ path = "{root / "vendor/gpui"}" }}\n\n'
        '[workspace.metadata.typos]',
    )
    manifest.write_text(text)
    shutil.copy(root / "Cargo.lock", output / "Cargo.lock")
    component = output / "crates/component/Cargo.toml"
    component.write_text(replace_once(
        component.read_text(), r'\[dev-dependencies\]',
        '[dev-dependencies]\ngpui_platform = { workspace = true, features = ["font-kit", "runtime_shaders"] }',
    ))
    fixture = output / "crates/component/examples/gpuio_table_probe.rs"
    fixture.parent.mkdir(exist_ok=True)
    shutil.copy(root / "docs/evidence/fixtures/table_adapter_probe.rs", fixture)
    if args.macro_fallback:
        macro = output / "crates/component-macros/src/crate_path.rs"
        macro.write_text(replace_once(
            macro.read_text(), r'Err\(kit_error\) => crate_name\("gpui-pre"\)',
            'Err(kit_error) => crate_name("gpui-pre")\n            .or_else(|_| crate_name("gpui"))',
        ))
    print(f"Created {output.relative_to(root)}")
    print(f"Source: {KIT_REV}; GPUI: {GPUI_REV}; macro fallback: {args.macro_fallback}")
    print("The seed lock needs isolated dependency resolution on the first Cargo command.")
    print("Run Cargo through ./scripts/gpuio exec, with --manifest-path pointing here and -j2.")
    print("Serialize with other Cargo/Dune builds; the wrapper shares this checkout's target cache.")


if __name__ == "__main__":
    main()
