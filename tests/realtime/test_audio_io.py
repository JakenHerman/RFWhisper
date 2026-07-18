"""PortAudio wrapper tests, driven by a fake backend.

CI runners have no sound card, so a stand-in ``sounddevice`` module provides the device
tables and lets tests fire callbacks by hand. Tests that need a real device are marked
``realhw`` and skip everywhere else.
"""

from __future__ import annotations

import contextlib
import sys
import types
import warnings
from collections.abc import Iterator
from typing import Any

import numpy as np
import pytest

from rfwhisper.realtime.audio_io import (
    STREAM_BLOCKSIZE,
    STREAM_SR_HZ,
    AudioDevice,
    InputStream,
    OutputStream,
    input_devices,
    list_devices,
    output_devices,
)

DEVICES: list[dict[str, Any]] = [
    {
        "name": "Microphone (USB Audio)",
        "hostapi": 0,
        "max_input_channels": 2,
        "max_output_channels": 0,
        "default_samplerate": 48000.0,
    },
    {
        "name": "CABLE Input (VB-Audio)",
        "hostapi": 1,
        "max_input_channels": 0,
        "max_output_channels": 2,
        "default_samplerate": 44100.0,
    },
    {
        "name": "Headset",
        "hostapi": 1,
        "max_input_channels": 1,
        "max_output_channels": 2,
        "default_samplerate": 48000.0,
    },
]


@contextlib.contextmanager
def warnings_as_errors() -> Iterator[None]:
    """Assert that a block emits no warnings at all."""
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        yield


class FakeStream:
    """Stands in for sd.InputStream / sd.OutputStream, recording its lifecycle."""

    instances: list[FakeStream] = []

    def __init__(self, **kwargs: Any) -> None:
        self.kwargs = kwargs
        self.callback = kwargs["callback"]
        self.started = False
        self.closed = False
        FakeStream.instances.append(self)

    def start(self) -> None:
        self.started = True

    def stop(self) -> None:
        self.started = False

    def close(self) -> None:
        self.closed = True

    def fire(self, frames: int = STREAM_BLOCKSIZE, status: object = "", data: Any = None) -> Any:
        """Invoke the registered callback the way PortAudio would."""
        buffer = np.zeros((frames, 1), dtype=np.float32) if data is None else data
        self.callback(buffer, frames, object(), status)
        return buffer


@pytest.fixture(autouse=True)
def fake_sounddevice(monkeypatch: pytest.MonkeyPatch) -> types.ModuleType:
    """Install a fake `sounddevice` for the duration of each test."""
    FakeStream.instances = []
    module = types.ModuleType("sounddevice")
    module.query_devices = lambda: DEVICES  # type: ignore[attr-defined]
    module.query_hostapis = lambda: [{"name": "MME"}, {"name": "WASAPI"}]  # type: ignore[attr-defined]
    module.InputStream = FakeStream  # type: ignore[attr-defined]
    module.OutputStream = FakeStream  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "sounddevice", module)
    return module


# --- Device enumeration ---------------------------------------------------------------


def test_list_devices_returns_at_least_one_device() -> None:
    """Acceptance: enumeration works on every CI runner via the fake backend."""
    devices = list_devices()
    assert len(devices) >= 1
    assert all(isinstance(d, AudioDevice) for d in devices)


def test_devices_carry_index_name_hostapi_channels_and_rate() -> None:
    first = list_devices()[0]
    assert first.index == 0
    assert first.name == "Microphone (USB Audio)"
    assert first.host_api == "MME"
    assert first.max_input_channels == 2
    assert first.max_output_channels == 0
    assert first.default_samplerate == 48000.0


def test_host_api_index_is_resolved_to_a_name() -> None:
    assert list_devices()[1].host_api == "WASAPI"


def test_unknown_host_api_index_does_not_crash_enumeration(
    fake_sounddevice: types.ModuleType,
) -> None:
    """A device pointing at a host API that isn't in the table still enumerates."""
    fake_sounddevice.query_devices = lambda: [{**DEVICES[0], "hostapi": 99}]  # type: ignore[attr-defined]
    assert list_devices()[0].host_api == "?"


def test_input_and_output_filters_split_on_channel_counts() -> None:
    assert [d.name for d in input_devices()] == ["Microphone (USB Audio)", "Headset"]
    assert [d.name for d in output_devices()] == ["CABLE Input (VB-Audio)", "Headset"]


def test_is_input_and_is_output_reflect_channels() -> None:
    mic, cable, headset = list_devices()
    assert mic.is_input and not mic.is_output
    assert cable.is_output and not cable.is_input
    assert headset.is_input and headset.is_output


# --- Stream format --------------------------------------------------------------------


def test_streams_pin_the_v01_format() -> None:
    """Mono float32 at 48 kHz in 480-frame blocks, so nothing has to re-buffer."""
    with InputStream(lambda block: None) as stream:
        kwargs = FakeStream.instances[-1].kwargs
        assert kwargs["channels"] == 1
        assert kwargs["samplerate"] == STREAM_SR_HZ == 48_000
        assert kwargs["blocksize"] == STREAM_BLOCKSIZE == 480
        assert kwargs["dtype"] == "float32"
        assert stream.active


def test_device_index_is_passed_through() -> None:
    with InputStream(lambda block: None, device=2):
        assert FakeStream.instances[-1].kwargs["device"] == 2


def test_rejects_nonsense_stream_parameters() -> None:
    with pytest.raises(ValueError, match="blocksize must be positive"):
        InputStream(lambda block: None, blocksize=0)
    with pytest.raises(ValueError, match="samplerate must be positive"):
        OutputStream(lambda block: None, samplerate=-1)


# --- Lifecycle ------------------------------------------------------------------------


def test_open_close_reopen_100_times_leaks_nothing() -> None:
    """Acceptance: 100 open/close cycles, every stream closed, none left dangling."""
    for _ in range(100):
        stream = InputStream(lambda block: None, device=0)
        stream.start()
        assert stream.active
        stream.stop()
        assert not stream.active

    assert len(FakeStream.instances) == 100
    assert all(s.closed for s in FakeStream.instances)
    assert not any(s.started for s in FakeStream.instances)


def test_stop_is_idempotent() -> None:
    stream = InputStream(lambda block: None)
    stream.start()
    stream.stop()
    stream.stop()
    assert not stream.active


def test_starting_twice_is_an_error() -> None:
    stream = InputStream(lambda block: None)
    stream.start()
    try:
        with pytest.raises(RuntimeError, match="already started"):
            stream.start()
    finally:
        stream.stop()


def test_context_manager_closes_on_exception() -> None:
    """A raising body must still release the device handle."""
    with pytest.raises(ValueError):
        with InputStream(lambda block: None):
            raise ValueError("boom")
    assert FakeStream.instances[-1].closed


# --- Callback behaviour ---------------------------------------------------------------


def test_input_callback_receives_mono_blocks() -> None:
    seen: list[np.ndarray] = []
    with InputStream(lambda block: seen.append(block.copy())):
        data = np.arange(STREAM_BLOCKSIZE, dtype=np.float32).reshape(-1, 1)
        FakeStream.instances[-1].fire(data=data)

    assert len(seen) == 1
    assert seen[0].shape == (STREAM_BLOCKSIZE,)
    assert np.array_equal(seen[0], np.arange(STREAM_BLOCKSIZE, dtype=np.float32))


def test_output_callback_fills_the_buffer_in_place() -> None:
    """No allocation per block: the callback writes into PortAudio's own buffer."""

    def fill(block: np.ndarray) -> None:
        block[:] = 0.25

    with OutputStream(fill):
        written = FakeStream.instances[-1].fire()

    assert np.allclose(written[:, 0], 0.25)


def test_output_callback_receives_a_view_not_a_copy() -> None:
    """If it were a copy, the in-place contract would silently emit silence."""
    captured: list[np.ndarray] = []
    with OutputStream(lambda block: captured.append(block)):
        buffer = np.zeros((STREAM_BLOCKSIZE, 1), dtype=np.float32)
        FakeStream.instances[-1].fire(data=buffer)
        captured[0][:] = 1.0
        assert np.allclose(buffer[:, 0], 1.0)


def test_blocks_are_counted() -> None:
    with InputStream(lambda block: None) as stream:
        for _ in range(5):
            FakeStream.instances[-1].fire()
        assert stream.stats.blocks == 5


# --- Failure handling -----------------------------------------------------------------


def test_a_raising_consumer_does_not_kill_the_stream() -> None:
    """A glitch beats a dead stream — count the drop and keep going."""

    def explode(block: np.ndarray) -> None:
        raise RuntimeError("consumer bug")

    with InputStream(explode) as stream:
        for _ in range(3):
            FakeStream.instances[-1].fire()
        assert stream.stats.dropped_blocks == 3
        assert stream.active


def test_output_emits_silence_when_the_consumer_raises() -> None:
    def explode(block: np.ndarray) -> None:
        block[:] = 0.9
        raise RuntimeError("consumer bug")

    with OutputStream(explode) as stream:
        written = FakeStream.instances[-1].fire()
        assert stream.stats.dropped_blocks == 1

    assert np.allclose(written, 0.0)


# --- Xrun telemetry -------------------------------------------------------------------


def test_xruns_are_counted_with_device_name_and_timestamp() -> None:
    with InputStream(lambda block: None, device=0) as stream:
        FakeStream.instances[-1].fire(status="input overflow")

        assert stream.stats.xruns == 1
        event = stream.stats.events[0]
        assert event.device == "Microphone (USB Audio)"
        assert event.status == "input overflow"
        assert event.wall_clock
        assert event.monotonic_s > 0.0


def test_poll_xruns_warns_once_per_event() -> None:
    """Acceptance: xruns surface as warnings naming the device and the time."""
    with InputStream(lambda block: None, device=0) as stream:
        FakeStream.instances[-1].fire(status="input overflow")

        with pytest.warns(RuntimeWarning, match="audio xrun on Microphone"):
            assert len(stream.poll_xruns()) == 1

        # Already reported — polling again must stay quiet.
        with warnings_as_errors():
            assert stream.poll_xruns() == []


def test_close_reports_outstanding_xruns() -> None:
    """Nothing gets silently lost if the owner never polls."""
    stream = InputStream(lambda block: None, device=0)
    stream.start()
    FakeStream.instances[-1].fire(status="input overflow")
    with pytest.warns(RuntimeWarning, match="audio xrun"):
        stream.stop()


def test_retained_events_are_capped_but_the_counter_is_not() -> None:
    """A continuously xrunning stream must not grow the event list without bound."""
    from rfwhisper.realtime.audio_io import MAX_RETAINED_EVENTS

    with InputStream(lambda block: None, device=0) as stream:
        for _ in range(MAX_RETAINED_EVENTS + 50):
            FakeStream.instances[-1].fire(status="overflow")

        assert stream.stats.xruns == MAX_RETAINED_EVENTS + 50
        assert len(stream.stats.events) == MAX_RETAINED_EVENTS


def test_xrun_rate_is_a_fraction_of_blocks() -> None:
    with InputStream(lambda block: None) as stream:
        for i in range(4):
            FakeStream.instances[-1].fire(status="overflow" if i < 1 else "")
        assert stream.stats.xrun_rate == pytest.approx(0.25)


def test_device_name_is_resolved_before_any_callback_runs(
    fake_sounddevice: types.ModuleType,
) -> None:
    """The xrun path must not enumerate devices on the realtime thread.

    Enumeration is made to explode after start(); a callback that still reports the
    right device name proves the lookup happened at start, not in the callback.
    """
    stream = InputStream(lambda block: None, device=0)
    stream.start()

    def explode() -> list[dict[str, Any]]:
        raise AssertionError("device enumeration must not happen on the audio thread")

    fake_sounddevice.query_devices = explode  # type: ignore[attr-defined]
    FakeStream.instances[-1].fire(status="input overflow")

    assert stream.stats.events[0].device == "Microphone (USB Audio)"


def test_xrun_rate_of_an_idle_stream_is_zero() -> None:
    with InputStream(lambda block: None) as stream:
        assert stream.stats.xrun_rate == 0.0


# --- Missing PortAudio ----------------------------------------------------------------


def test_missing_portaudio_gives_an_actionable_message(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "sounddevice", None)
    with pytest.raises(RuntimeError, match=r"pip install -e '\.\[audio\]'"):
        list_devices()


# --- Real hardware --------------------------------------------------------------------


@pytest.mark.realhw
def test_enumerates_a_real_device(monkeypatch: pytest.MonkeyPatch) -> None:
    """Only meaningful on a machine with a sound card.

    Skips rather than fails without one — CI runners have no audio hardware, and the
    marker alone would not save a developer running the full suite on a headless box.
    """
    monkeypatch.delitem(sys.modules, "sounddevice", raising=False)
    pytest.importorskip("sounddevice", reason="needs the [audio] extra and a real device")
    try:
        devices = list_devices()
    except RuntimeError as exc:
        pytest.skip(f"no usable PortAudio backend: {exc}")
    assert len(devices) >= 1
