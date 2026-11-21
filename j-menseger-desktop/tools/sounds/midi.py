"""Escritor mínimo de arquivos MIDI (formato 0), com tempo em milissegundos."""
import struct


def _vlq(n):
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.append(0x80 | (n & 0x7F))
        n >>= 7
    return bytes(reversed(out))


class Song:
    """Eventos em ms. 1 tick = 1 ms (PPQ 1000, tempo 1 000 000 µs por semínima)."""

    def __init__(self):
        self.ev = []  # (ms, ordem, bytes)
        self.n = 0

    def _add(self, ms, data):
        self.n += 1
        self.ev.append((max(0, int(round(ms))), self.n, bytes(data)))

    def program(self, ms, ch, prog, bank=0):
        self.cc(ms, ch, 0, bank)
        self.cc(ms, ch, 32, 0)
        self._add(ms, [0xC0 | ch, prog])

    def cc(self, ms, ch, num, val):
        self._add(ms, [0xB0 | ch, num, max(0, min(127, int(val)))])

    def note(self, ms, ch, key, vel, dur):
        self._add(ms, [0x90 | ch, key, max(1, min(127, int(vel)))])
        self._add(ms + dur, [0x80 | ch, key, 0])

    def bend(self, ms, ch, semis, rng=2):
        v = int(8192 + max(-1, min(1, semis / rng)) * 8191)
        self._add(ms, [0xE0 | ch, v & 0x7F, (v >> 7) & 0x7F])

    def bend_range(self, ms, ch, semis):
        self.cc(ms, ch, 101, 0); self.cc(ms, ch, 100, 0)
        self.cc(ms, ch, 6, semis); self.cc(ms, ch, 38, 0)
        self.cc(ms, ch, 101, 127); self.cc(ms, ch, 100, 127)

    def ramp(self, ms0, ms1, fn, steps=None):
        """Chama fn(ms, x) com x de 0 a 1, a cada ~10 ms."""
        steps = steps or max(2, int((ms1 - ms0) / 10))
        for i in range(steps + 1):
            x = i / steps
            fn(ms0 + (ms1 - ms0) * x, x)

    def save(self, path, end_ms):
        self._add(end_ms, [0xFF, 0x2F, 0x00])
        ev = sorted(self.ev, key=lambda e: (e[0], e[1]))
        trk = bytearray(_vlq(0) + b'\xff\x51\x03' + (1000000).to_bytes(3, 'big'))
        last = 0
        for ms, _, data in ev:
            if data[0] == 0xFF and data[1] == 0x2F:
                continue
            trk += _vlq(ms - last) + data
            last = ms
        trk += _vlq(max(0, end_ms - last)) + b'\xff\x2f\x00'
        with open(path, 'wb') as f:
            f.write(b'MThd' + struct.pack('>IHHH', 6, 0, 1, 1000))
            f.write(b'MTrk' + struct.pack('>I', len(trk)) + trk)
