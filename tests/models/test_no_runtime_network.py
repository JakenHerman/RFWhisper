"""Import audit: no runtime module may touch the network (AGENTS Prime Directive 4).

Local-first is a promise to operators, not an aspiration. The cheapest way to break it is
an innocuous module-scope call — a version check, a telemetry ping, a lazily-fetched
config — added years from now by someone who never read the directive. So this test
imports every runtime module in a subprocess with sockets disabled and fails if any of
them reaches out.

A subprocess is necessary: by the time this test runs, pytest has already imported half
the tree, so patching sockets in-process would prove nothing.
"""

from __future__ import annotations

import subprocess
import sys
import textwrap

# Everything reachable from the audio path or the CLI's default commands. `models.fetch`
# is deliberately absent — downloading is its entire job.
RUNTIME_MODULES = [
    "rfwhisper",
    "rfwhisper.constants",
    "rfwhisper.cli",
    "rfwhisper.dsp.features",
    "rfwhisper.dsp.metrics",
    "rfwhisper.dsp.resample",
    "rfwhisper.denoise.engine",
    "rfwhisper.models.base",
    "rfwhisper.models.null_model",
    "rfwhisper.models.registry",
]

_GUARD = """
import socket, sys

class NetworkAccessDenied(RuntimeError):
    pass

def _deny(*args, **kwargs):
    raise NetworkAccessDenied("runtime module attempted network access")

socket.socket = _deny
socket.create_connection = _deny
socket.getaddrinfo = _deny

for name in {modules!r}:
    try:
        __import__(name)
    except ImportError as exc:
        # An optional extra being absent is fine; a network attempt is not.
        print(f"SKIP {{name}}: {{exc}}")
    except NetworkAccessDenied:
        print(f"NETWORK {{name}}")
        sys.exit(1)

print("CLEAN")
"""


def test_runtime_imports_make_no_network_calls() -> None:
    """Acceptance (#12): no network calls at runtime, enforced rather than documented."""
    script = _GUARD.format(modules=RUNTIME_MODULES)
    result = subprocess.run(
        [sys.executable, "-c", textwrap.dedent(script)],
        capture_output=True,
        text=True,
        timeout=120,
    )

    assert result.returncode == 0, (
        f"a runtime module tried to use the network:\n{result.stdout}\n{result.stderr}"
    )
    assert "CLEAN" in result.stdout
    assert "NETWORK" not in result.stdout


def test_the_guard_actually_catches_a_network_call() -> None:
    """A guard that cannot fail proves nothing — verify it trips on a real attempt."""
    script = _GUARD.format(modules=["socket_probe"]) + textwrap.dedent(
        """
        import socket
        try:
            socket.socket()
        except NetworkAccessDenied:
            print("GUARD_WORKS")
        else:
            sys.exit(2)
        """
    )
    result = subprocess.run(
        [sys.executable, "-c", textwrap.dedent(script)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert "GUARD_WORKS" in result.stdout, result.stdout + result.stderr
