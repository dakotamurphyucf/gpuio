"""Hosted diagnostic collection only; a green job does not qualify presentation."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Hosted desktop only")
output = Path(".cache/metal-arm-pacing").resolve()
output.mkdir(parents=True, exist_ok=True)
source = Path("scripts/diagnose_metal_arm_serial.swift").read_text()
assert source.count("    var startupWaitTicks = 0") == 1
assert source.count("        // Diagnostic: one in-flight drawable") == 1
delayed = source.replace(
    "    var startupWaitTicks = 0",
    "    var startupWaitTicks = 0\n    var visibleSince: Date?",
).replace(
    "        // Diagnostic: one in-flight drawable",
    """        // Fixed startup delay after first active/visible observation. The original
        // twelve-second deadline and every submitted frame remain in force.
        if visibleSince == nil { visibleSince = Date() }
        if Date().timeIntervalSince(visibleSince!) < 5.0 { return }
        // Diagnostic: one in-flight drawable""",
)
# Ensure neither variant modifies acceptance, callback collection, or teardown.
assert delayed.split("    func finish(")[1] == source.split("    func finish(")[1]
summary = []
for mode, text, delay in [("serial", source, 0), ("serial-delayed", delayed, 5)]:
    directory = output / mode
    directory.mkdir(exist_ok=False)
    swift = directory / "probe.swift"
    swift.write_text(text)
    binary = directory / "probe"
    record = {"mode": mode, "startup_delay_seconds": delay,
              "source_sha256": hashlib.sha256(swift.read_bytes()).hexdigest()}
    with (directory / "build.log").open("w") as log:
        subprocess.run(["xcrun", "swiftc", "-O", str(swift), "-o", str(binary)],
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
    record["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
    awake = subprocess.Popen(["caffeinate", "-di", "-w", str(os.getpid())])
    try:
        with (directory / "report.json").open("w") as out, (directory / "stderr.log").open("w") as err:
            try:
                result = subprocess.run([str(binary)], cwd=directory, stdout=out, stderr=err, timeout=30)
                record["returncode"] = result.returncode
            except subprocess.TimeoutExpired:
                record["timeout"] = True
        try:
            report = json.loads((directory / "report.json").read_text())
            frames = report.get("frames", [])
            record.update(complete=report.get("complete"), device=report.get("device"),
                          failures=report.get("failures"), frames=len(frames),
                          positive_presentations=sum(f.get("presented_s", 0) > 0 for f in frames),
                          gpu_completions=sum(f.get("gpu_status") == 4 for f in frames))
        except json.JSONDecodeError as error:
            record["parse_error"] = str(error)
    finally:
        awake.terminate()
        awake.wait(timeout=5)
        binary.unlink(missing_ok=True)
    summary.append(record)
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
