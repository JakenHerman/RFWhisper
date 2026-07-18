"""PortAudio capture and playback primitives.

Thin, callback-based wrappers over ``sounddevice``. Primitives only — the processor
topology that wires an input to a model to an output is #14, and the end-to-end latency
gate is #22.

v0.1 pins the stream format to mono float32 at 48 kHz with a 480-sample (10 ms) block, so
the hop the model wants is the hop PortAudio delivers and nothing in the pipeline has to
re-buffer. Re-buffering is latency, and A4 gives us a 100 ms p99 budget for the whole
chain.

``sounddevice`` lives in the ``[audio]`` extra and is imported lazily, so importing this
module — for the dataclasses, say — never requires PortAudio to be installed.
"""

from __future__ import annotations

import time
import warnings
from collections.abc import Callable
from dataclasses import dataclass, field
from types import ModuleType, TracebackType
from typing import Any

import numpy as np
from numpy.typing import NDArray

from rfwhisper.constants import DEFAULT_BLOCKSIZE, NATIVE_DFN_SR_HZ

# v0.1 stream format. Not configurable on purpose: every other component assumes it.
STREAM_SR_HZ: int = NATIVE_DFN_SR_HZ
STREAM_CHANNELS: int = 1
STREAM_DTYPE: str = "float32"
STREAM_BLOCKSIZE: int = DEFAULT_BLOCKSIZE

#: Callback signature. Receives a mono float32 block; fills it in place for output.
BlockCallback = Callable[[NDArray[np.float32]], None]


def _sounddevice() -> ModuleType:
    """Import ``sounddevice``, with a message that says what to install if it's missing."""
    try:
        import sounddevice as sd
    except (ImportError, OSError) as exc:
        # OSError: the Python package is installed but the PortAudio system library
        # behind it is not — common on bare Linux containers.
        raise RuntimeError(
            "sounddevice/PortAudio unavailable; install the audio extra: pip install -e '.[audio]'"
        ) from exc
    module: ModuleType = sd
    return module


@dataclass(frozen=True)
class AudioDevice:
    """One PortAudio device as reported by the host API."""

    index: int
    name: str
    host_api: str
    max_input_channels: int
    max_output_channels: int
    default_samplerate: float

    @property
    def is_input(self) -> bool:
        """True if this device can capture."""
        return self.max_input_channels > 0

    @property
    def is_output(self) -> bool:
        """True if this device can play back."""
        return self.max_output_channels > 0


def list_devices() -> list[AudioDevice]:
    """Enumerate every audio device PortAudio can see, in device-index order."""
    sd = _sounddevice()
    hostapis = [dict(a) for a in sd.query_hostapis()]
    devices: list[AudioDevice] = []
    for index, raw in enumerate(sd.query_devices()):
        info = dict(raw)
        api_index = int(info.get("hostapi", -1))
        api_name = str(hostapis[api_index]["name"]) if 0 <= api_index < len(hostapis) else "?"
        devices.append(
            AudioDevice(
                index=index,
                name=str(info.get("name", "?")),
                host_api=api_name,
                max_input_channels=int(info.get("max_input_channels", 0)),
                max_output_channels=int(info.get("max_output_channels", 0)),
                default_samplerate=float(info.get("default_samplerate", 0.0)),
            )
        )
    return devices


def input_devices() -> list[AudioDevice]:
    """Devices that can capture."""
    return [d for d in list_devices() if d.is_input]


def output_devices() -> list[AudioDevice]:
    """Devices that can play back."""
    return [d for d in list_devices() if d.is_output]


@dataclass
class XrunEvent:
    """One PortAudio over/underflow, with enough context to file a bug about it."""

    device: str
    monotonic_s: float
    wall_clock: str
    status: str


@dataclass
class StreamStats:
    """Telemetry counters for one stream. The GUI (#74) reads these live.

    ``blocks`` counts every callback; ``xruns`` counts the ones PortAudio flagged, and
    ``dropped_blocks`` the ones where the user callback raised. A raising callback is
    survivable — we emit silence for that block and keep the stream up — but it is a bug,
    so it gets counted rather than swallowed.
    """

    blocks: int = 0
    xruns: int = 0
    dropped_blocks: int = 0
    events: list[XrunEvent] = field(default_factory=list)

    @property
    def xrun_rate(self) -> float:
        """Fraction of callbacks that reported an over/underflow."""
        return self.xruns / self.blocks if self.blocks else 0.0


# Cap on retained xrun events. A stream that is xrunning continuously would otherwise
# grow this list without bound — and the first few events say everything the last ten
# thousand would.
MAX_RETAINED_EVENTS: int = 256


class _CallbackStream:
    """Shared open/close/telemetry behaviour for the input and output wrappers.

    Xruns are *counted* inside the audio callback and *reported* outside it. Emitting a
    warning from a PortAudio callback means allocating and taking the GIL on the realtime
    thread, which is a good way to cause the very dropout you are trying to report. So the
    callback appends a small record and :meth:`poll_xruns` — called by the owner, or
    automatically on close — turns those into warnings.
    """

    def __init__(self, device: int | None, blocksize: int, samplerate: int) -> None:
        if blocksize <= 0:
            raise ValueError("blocksize must be positive")
        if samplerate <= 0:
            raise ValueError("samplerate must be positive")
        self._device = device
        self._blocksize = blocksize
        self._samplerate = samplerate
        self._stream: Any = None
        self._reported = 0
        self._device_name: str | None = None
        self.stats = StreamStats()

    def _resolve_device_name(self) -> str:
        """Look the device name up. Queries PortAudio — never call from a callback."""
        if self._device is None:
            return "default"
        try:
            for candidate in list_devices():
                if candidate.index == self._device:
                    return candidate.name
        except RuntimeError:  # pragma: no cover - only without PortAudio
            pass
        return f"device {self._device}"

    @property
    def device_name(self) -> str:
        """Human-readable name of the device this stream is bound to.

        Resolved once at :meth:`start` and cached, because the xrun path reads it from
        the audio callback — enumerating devices there would mean an allocation and a
        PortAudio round-trip on the realtime thread, causing the very dropout it is
        trying to report.
        """
        if self._device_name is None:
            self._device_name = self._resolve_device_name()
        return self._device_name

    @property
    def blocksize(self) -> int:
        """Frames per callback."""
        return self._blocksize

    @property
    def samplerate(self) -> int:
        """Stream sample rate in Hz."""
        return self._samplerate

    @property
    def active(self) -> bool:
        """True between :meth:`start` and :meth:`stop`/:meth:`close`."""
        return self._stream is not None

    def _note_status(self, status: object) -> None:
        """Record a PortAudio status flag. Called on the audio thread — keep it cheap.

        Reads the cached device name rather than resolving it; see :attr:`device_name`.
        """
        self.stats.xruns += 1
        if len(self.stats.events) < MAX_RETAINED_EVENTS:
            self.stats.events.append(
                XrunEvent(
                    device=self._device_name or "unknown",
                    monotonic_s=time.monotonic(),
                    wall_clock=time.strftime("%Y-%m-%dT%H:%M:%S"),
                    status=str(status),
                )
            )

    def poll_xruns(self) -> list[XrunEvent]:
        """Emit a warning per xrun recorded since the last poll, and return them."""
        fresh = self.stats.events[self._reported :]
        self._reported = len(self.stats.events)
        for event in fresh:
            warnings.warn(
                f"audio xrun on {event.device} at {event.wall_clock}: {event.status}",
                RuntimeWarning,
                stacklevel=2,
            )
        return fresh

    def start(self) -> None:
        """Open and start the underlying PortAudio stream."""
        if self._stream is not None:
            raise RuntimeError("stream is already started")
        # Resolve the name now, off the audio thread, so the xrun path can just read it.
        self._device_name = self._resolve_device_name()
        self._stream = self._open()
        self._stream.start()

    def stop(self) -> None:
        """Stop and close the stream, reporting any xruns that accumulated.

        Idempotent, and safe to call from a ``finally`` — a half-open stream still
        releases its device handle.
        """
        stream, self._stream = self._stream, None
        if stream is not None:
            try:
                stream.stop()
            finally:
                stream.close()
        self.poll_xruns()

    close = stop

    def _open(self) -> Any:  # pragma: no cover - overridden
        raise NotImplementedError

    def __enter__(self) -> Any:
        self.start()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None:
        self.stop()


class InputStream(_CallbackStream):
    """Capture mono float32 blocks and hand each one to ``callback``.

    The callback runs on PortAudio's realtime thread: it must not block, allocate
    heavily, or do I/O. Hand the block to a queue and get out (see
    :mod:`rfwhisper.realtime.processor`).

    **The block is a view into a buffer PortAudio reuses**, not a copy. It is valid only
    for the duration of the call. A consumer that keeps a reference — queues it, stores
    it, slices it lazily — will find the contents rewritten underneath it a few
    milliseconds later. Call ``.copy()`` if the data must outlive the callback.
    """

    def __init__(
        self,
        callback: BlockCallback,
        device: int | None = None,
        blocksize: int = STREAM_BLOCKSIZE,
        samplerate: int = STREAM_SR_HZ,
    ) -> None:
        super().__init__(device, blocksize, samplerate)
        self._callback = callback

    def _on_audio(self, indata: Any, frames: int, timeinfo: object, status: object) -> None:
        self.stats.blocks += 1
        if status:
            self._note_status(status)
        try:
            # A view into PortAudio's buffer, not a copy — see the class docstring.
            self._callback(indata[:, 0])
        except Exception:
            # Never let a consumer bug tear down the audio thread; count it instead.
            self.stats.dropped_blocks += 1

    def _open(self) -> Any:
        sd = _sounddevice()
        return sd.InputStream(
            device=self._device,
            channels=STREAM_CHANNELS,
            samplerate=self._samplerate,
            blocksize=self._blocksize,
            dtype=STREAM_DTYPE,
            callback=self._on_audio,
        )


class OutputStream(_CallbackStream):
    """Ask ``callback`` to fill each outgoing mono float32 block, in place.

    Filling in place rather than returning an array keeps the realtime path free of
    per-callback allocations. If the callback raises, the block is emitted as silence and
    counted in ``stats.dropped_blocks`` — a glitch beats a dead stream.
    """

    def __init__(
        self,
        callback: BlockCallback,
        device: int | None = None,
        blocksize: int = STREAM_BLOCKSIZE,
        samplerate: int = STREAM_SR_HZ,
    ) -> None:
        super().__init__(device, blocksize, samplerate)
        self._callback = callback

    def _on_audio(self, outdata: Any, frames: int, timeinfo: object, status: object) -> None:
        self.stats.blocks += 1
        if status:
            self._note_status(status)
        try:
            # Hand out the buffer itself so the callback writes straight into it;
            # np.asarray on a column slice would copy and allocate every block.
            self._callback(outdata[:, 0])
        except Exception:
            self.stats.dropped_blocks += 1
            outdata[:] = 0.0

    def _open(self) -> Any:
        sd = _sounddevice()
        return sd.OutputStream(
            device=self._device,
            channels=STREAM_CHANNELS,
            samplerate=self._samplerate,
            blocksize=self._blocksize,
            dtype=STREAM_DTYPE,
            callback=self._on_audio,
        )
