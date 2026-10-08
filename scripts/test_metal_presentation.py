#!/usr/bin/env python3
"""Build/run the bounded two-window Metal hook qualification (macOS only).

No VoiceOver, accessibility automation or preference changes. The native probe
closes its windows. The parent also imposes a timeout and requires a fresh report;
an NSApplication exit(0) without completed assertions cannot pass.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, help="Use an already built probe")
    parser.add_argument("--idle-before-frames-ms", type=int, default=0,
                        help="Observe native idle-to-active behavior without the OCaml bridge (0..5000)")
    parser.add_argument("--allow-unavailable-timing", action="store_true",
                        help="Preview CI only: classify complete, retired all-zero callback reports as unavailable")
    args = parser.parse_args()
    if not 0 <= args.idle_before_frames_ms <= 5000:
        parser.error("Idle delay must be between 0 and 5000 ms")
    if sys.platform != "darwin":
        raise SystemExit("This qualification requires a macOS Metal desktop")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report = output / "report.json"
    if report.exists():
        raise SystemExit("Refusing to reuse an existing report; choose a fresh output directory")
    binary = args.binary
    if binary is None:
        command = [str(ROOT / "scripts/gpuio"), "exec", "cargo", "build", "-p", "gpuio-native",
                   "--locked", "--features", "presentation-diagnostics", "--test",
                   "native_metal_presentation", "--message-format=json", "-j", os.environ.get("GPUIO_JOBS", "2")]
        with (output / "build.jsonl").open("w") as stdout, (output / "build.log").open("w") as stderr:
            subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, check=True)
        artifacts = [json.loads(line) for line in (output / "build.jsonl").read_text().splitlines()]
        candidates = [item["executable"] for item in artifacts
                      if item.get("reason") == "compiler-artifact"
                      and item.get("target", {}).get("name") == "native_metal_presentation"
                      and item.get("executable")]
        if len(candidates) != 1:
            raise RuntimeError(f"Expected one current native probe artifact, got {candidates}")
        binary = Path(candidates[0])
    environment = {**os.environ, "GPUIO_PRESENTATION_REPORT": str(report),
                   "GPUIO_PRESENTATION_IDLE_MS": str(args.idle_before_frames_ms)}
    # subprocess.run kills and reaps the child on timeout; no detached process.
    with (output / "native.log").open("w") as log:
        result = subprocess.run([str(binary.resolve())], cwd=ROOT, env=environment,
                                stdout=log, stderr=subprocess.STDOUT, timeout=25 + args.idle_before_frames_ms / 1000, check=True)
    from metal_timing_availability import load_report
    data = load_report(report)
    if data.get("idle_before_frames_ms") != args.idle_before_frames_ms:
        raise RuntimeError("Native probe did not report the requested idle setup")
    if args.allow_unavailable_timing:
        from metal_timing_availability import classify_hook
        status = classify_hook(data)
        classification = {"status": status, "child_exit": result.returncode,
                          "scope": "Hosted preview timing availability; not physical workload acceptance"}
        (output / "availability.json").write_text(json.dumps(classification, indent=2) + "\n")
        print(json.dumps(classification))
        if status == "unavailable":
            print("::warning::Hosted GPUI Metal returned only zero presentation timestamps; timing unavailable. Physical qualification remains required.")
        return
    if data.get("schema") != 1 or data.get("kind") != "gpui_metal_hook_qualification" or data.get("passed") is not True:
        raise RuntimeError(f"Native qualification did not pass; inspect {report}")
    if len(data.get("sessions", [])) != 2:
        raise RuntimeError("Missing independent window results")
    print(json.dumps({"passed": True, "exit_code": result.returncode, "report": str(report),
                      "presented": [s["counts"]["presented"] for s in data["sessions"]]}))


if __name__ == "__main__":
    main()
