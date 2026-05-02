"""
Real-time duplex stream: avoid heavy work in PortAudio callback — use a worker thread.

Configure ORT with RFWHISPER_ORT_INTRA_OP; preallocate where the engine allows.
"""

from __future__ import annotations

import os
import queue
import threading

import numpy as np
import sounddevice as sd

from rfwhisper.constants import DEFAULT_BLOCKSIZE
from rfwhisper.denoise.engine import select_engine


def stream_denoise(
    in_dev: int | None,
    out_dev: int | None,
    model: str,
    blocksize: int = DEFAULT_BLOCKSIZE,
) -> None:
    """Duplex stream: callback enqueues; worker runs `DenoiseEngine.process`."""
    eng = select_engine(model)
    # Explicit None check — `in_dev or ...` would treat device index 0 as falsy and
    # silently query the default input device, causing a sample-rate mismatch with
    # the actual stream below.
    query_dev = in_dev if in_dev is not None else sd.default.device[0]
    dev_sr = int(sd.query_devices(query_dev, "input")["default_samplerate"])
    q_in: queue.Queue[np.ndarray] = queue.Queue(maxsize=32)
    q_out: queue.Queue[np.ndarray] = queue.Queue(maxsize=32)
    stop = threading.Event()

    def worker() -> None:
        while not stop.is_set():
            try:
                mono = q_in.get(timeout=0.2)
            except queue.Empty:
                continue
            y = eng.process(mono, dev_sr)
            y = np.asarray(y, dtype=np.float32)
            if y.size != mono.size:
                if y.size > mono.size:
                    y = y[: mono.size]
                else:
                    y = np.pad(y, (0, mono.size - y.size))
            if not stop.is_set():
                try:
                    q_out.put_nowait(y)
                except queue.Full:
                    pass

    th = threading.Thread(target=worker, name="rfwhisper-denoise", daemon=True)
    th.start()

    def callback(
        indata: np.ndarray,
        outdata: np.ndarray,
        frames: int,
        time: object,
        status: object,
    ) -> None:
        if status and os.environ.get("RFWHISPER_DEBUG"):
            print(status, flush=True)
        mono = indata[:, 0].copy()
        try:
            q_in.put_nowait(mono)
        except queue.Full:
            pass
        try:
            outdata[:, 0] = q_out.get_nowait()
        except queue.Empty:
            outdata[:] = 0

    try:
        with sd.Stream(
            device=(in_dev, out_dev),
            channels=1,
            samplerate=dev_sr,
            blocksize=blocksize,
            dtype="float32",
            callback=callback,
        ):
            print(
                f"Streaming (SR={dev_sr} Hz, block={blocksize}); Ctrl+C or Enter to stop.",
                flush=True,
            )
            if os.environ.get("CI") or os.environ.get("RFWHISPER_HEADLESS"):
                import time

                time.sleep(0.2)
            else:
                try:
                    input()
                except EOFError:
                    import time

                    time.sleep(10**9)
    except KeyboardInterrupt:
        pass
    finally:
        stop.set()
        th.join(timeout=2.0)
