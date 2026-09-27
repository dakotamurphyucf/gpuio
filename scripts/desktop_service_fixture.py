"""Disposable file handler and native Finder checks for Desktop Lab acceptance."""
from pathlib import Path
import os
import plistlib
import signal
import subprocess
import time

from test_agent_chat import Mac

LSREGISTER = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"


class ServiceFixture:
    def __init__(self, root, owned_pids):
        self.root = root
        self.owned_pids = owned_pids
        self.bundle = root / "Fixture Receiver.app"
        self.finder = None
        executable = self.bundle / "Contents/MacOS/gpuio-desktop"
        executable.parent.mkdir(parents=True)
        extension = "".join(c for c in root.name if c.isalnum())
        content_type = "com.gpuio.fixture." + extension
        self.path = root / ("Fixture résumé $()." + extension)
        self.path.write_text("Only this disposable fixture is opened.\n")
        with (self.bundle / "Contents/Info.plist").open("wb") as output:
            plistlib.dump({
                "CFBundleExecutable": executable.name,
                "CFBundleIdentifier": content_type,
                "CFBundleName": "GPUIO Fixture Receiver",
                "CFBundlePackageType": "APPL",
                "CFBundleVersion": "1",
                "LSUIElement": True,
                "CFBundleDocumentTypes": [{"CFBundleTypeName": "GPUIO test document",
                    "CFBundleTypeExtensions": [extension], "CFBundleTypeRole": "Viewer",
                    "LSHandlerRank": "Owner", "LSItemContentTypes": [content_type]}],
                "UTExportedTypeDeclarations": [{"UTTypeIdentifier": content_type,
                    "UTTypeConformsTo": ["public.data"],
                    "UTTypeTagSpecification": {"public.filename-extension": [extension]}}],
            }, output)
        source = Path(__file__).parent / "fixtures/desktop_open_target.m"
        subprocess.run(["xcrun", "clang", "-fobjc-arc", "-fblocks", "-framework", "Cocoa",
                        str(source), "-o", str(executable)], check=True, timeout=30)
        subprocess.run([LSREGISTER, "-f", str(self.bundle)], check=True, timeout=10)

    def exercise(self, send, wait_for):
        send("services/missing")
        wait_for("DESKTOP_LAB: service /missing (Error Unavailable)")
        send("services/undeclared")
        wait_for("DESKTOP_LAB: service /undeclared (Error Invalid_request)")
        send("services/unpackaged")
        wait_for("DESKTOP_LAB: service /unpackaged (Error Invalid_request)")
        send("services/open")
        wait_for("DESKTOP_LAB: service /open (Ok())")
        receipt = self.root / "opened-path.txt"
        deadline = time.monotonic() + 5
        while not receipt.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        assert os.path.samefile(receipt.read_text(), self.path), "Wrong file reached the OS-selected app"
        send("services/register")
        wait_for("DESKTOP_LAB: service /register (Ok())")
        # This invocation omits -a: registration must route the private test
        # scheme through Launch Services to the same application.
        subprocess.run(["/usr/bin/open", "-g", "-u", "gpuio-desktop-lab://document/registered"], check=True, timeout=5)
        wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/registered")
        send("services/reveal")
        wait_for("DESKTOP_LAB: service /reveal (Ok())")
        processes = subprocess.check_output(["ps", "-axo", "pid=,command="], text=True)
        for line in processes.splitlines():
            fields = line.strip().split(maxsplit=1)
            if len(fields) == 2 and fields[1] == "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder":
                self.finder = Mac(int(fields[0]))
                break
        assert self.finder is not None, "Finder was not running after reveal"
        node = self.finder.wait_find(self.root.name, "Fixture résumé $()", contains=True, search_files=True)
        self.finder.release(node)
        print("GPUIO_DESKTOP_SERVICES_OK: actual file consumer, missing/undeclared errors, default scheme routing, Finder fixture visibility")

    def close(self):
        try:
            if self.finder is not None:
                try:
                    window = self.finder.window(self.root.name)
                    if window:
                        self.finder.release(window)
                        self.finder.close(self.root.name)
                finally:
                    self.finder.release(self.finder.app)
        finally:
            for pid in self.owned_pids(self.bundle):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            subprocess.run([LSREGISTER, "-u", str(self.bundle)], check=True, timeout=10)
