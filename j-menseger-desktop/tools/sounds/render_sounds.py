"""Gera os sons do soundboard (web/sounds/*.ogg) a partir de amostras gravadas de instrumentos reais.

Fonte das amostras: MuseScore_General (MIT; partes CC0/domínio público) e FluidR3_GM (MIT).
No Ubuntu/Debian: apt install fluidsynth musescore-general-soundfont ffmpeg python3-numpy

Uso: python3 tools/sounds/render_sounds.py /caminho/MuseScore_General_Full.sf3 [saida]
Só o som de grilos é sintetizado (não existe amostra de grilo nos bancos livres).
"""
import json
import math
import os
import random
import subprocess
import sys
import tempfile
import wave

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from midi import Song  # noqa: E402

SR = 48000
DRUMS = 9


def drums(s, kit=0):
    s.program(0, DRUMS, kit, bank=128)


def setup(s, ch, prog, bank=0, vol=110, rev=50, pan=64):
    s.program(0, ch, prog, bank)
    s.cc(0, ch, 7, vol)
    s.cc(0, ch, 10, pan)
    s.cc(0, ch, 91, rev)
    s.cc(0, ch, 93, 0)
    s.cc(0, ch, 11, 127)


def expr_ramp(s, ch, t0, t1, a, b):
    s.ramp(t0, t1, lambda ms, x: s.cc(ms, ch, 11, a + (b - a) * x))


# ---------------------------------------------------------------- sons

def aplausos():
    s = Song()
    setup(s, 0, 126, rev=55)
    for key, vel, at in ((60, 118, 0), (55, 100, 90), (64, 96, 170), (58, 90, 260)):
        s.note(at, 0, key, vel, 3500 - at)
    expr_ramp(s, 0, 0, 450, 70, 127)
    expr_ramp(s, 0, 2500, 3500, 127, 0)
    return s, 3900


def palmas():
    s = Song()
    drums(s, 8)  # kit "Room": palmas com mais ar de sala
    s.cc(0, DRUMS, 91, 115)
    s.cc(0, DRUMS, 7, 120)
    for at, vel in ((0, 112), (660, 108), (1290, 113), (1910, 110), (2520, 118)):
        s.note(at, DRUMS, 39, vel, 200)
    return s, 3700


def badumtss():
    s = Song()
    drums(s, 0)
    s.cc(0, DRUMS, 91, 40)
    s.note(0, DRUMS, 38, 96, 120)        # "ba": caixa
    s.note(150, DRUMS, 43, 116, 200)     # "dum": surdo
    s.note(410, DRUMS, 49, 122, 300)     # "tss": prato de ataque
    s.note(410, DRUMS, 36, 120, 200)     #        + bumbo
    return s, 2700


def rufar():
    s = Song()
    drums(s, 0)
    s.cc(0, DRUMS, 91, 45)
    rnd = random.Random(7)
    t, end = 0, 2300
    while t < end:
        x = t / end
        s.note(t + rnd.uniform(-3, 3), DRUMS, 38, 50 + 66 * x ** 1.4 + rnd.uniform(-5, 5), 30)
        t += 37
    s.note(2340, DRUMS, 49, 125, 300)
    s.note(2340, DRUMS, 57, 104, 300)
    s.note(2340, DRUMS, 36, 127, 200)
    return s, 4400


def trombone():
    s = Song()
    setup(s, 0, 57, rev=40)
    notes = ((58, 0, 430), (57, 450, 430), (56, 900, 430), (55, 1350, 1750))
    for key, at, dur in notes:
        s.note(at, 0, key, 104, dur)
        expr_ramp(s, 0, at, at + 110, 72, 127)
        if dur < 1000:
            expr_ramp(s, 0, at + 110, at + dur, 127, 98)
    # última nota: "uá-uá" (surdina) + vibrato largo e some
    def wah(ms, x):
        t = (ms - 1500) / 1000
        s.cc(ms, 0, 11, 104 + 23 * math.sin(2 * math.pi * 5.2 * t))
        s.bend(ms, 0, 0.16 * math.sin(2 * math.pi * 5.2 * t + 1.2))
    s.ramp(1500, 2750, wah)
    expr_ramp(s, 0, 2750, 3100, 104, 0)
    s.bend(3110, 0, 0)
    return s, 3400


def suspense():
    s = Song()
    setup(s, 0, 61, rev=75)            # naipe de metais
    setup(s, 1, 48, rev=75, vol=100)   # cordas (ataque rápido)
    setup(s, 2, 47, rev=70, vol=120)   # tímpanos
    setup(s, 3, 44, rev=80, vol=96)    # cordas em trêmolo
    setup(s, 4, 116, bank=8, rev=70)   # bumbo de concerto
    drums(s, 0)
    s.cc(0, DRUMS, 91, 70)
    stab = (48, 51, 55, 60)
    for at in (0, 330):
        for k in stab:
            s.note(at, 0, k, 112, 190)
            s.note(at, 1, k, 100, 190)
        s.note(at, 2, 48, 112, 300)
    hit = (47, 50, 53, 56, 59)
    for k in hit:
        s.note(700, 0, k, 122, 2350)
        s.note(700, 3, k, 104, 2350)
    s.note(700, 0, 35, 118, 2350)
    s.note(700, 4, 36, 127, 1500)
    s.note(700, DRUMS, 49, 112, 300)
    t, rnd = 760, random.Random(3)
    while t < 2800:
        s.note(t, 2, 47, 62 + 50 * (t - 760) / 2040 + rnd.uniform(-4, 4), 70)
        t += 58
    for ch in (0, 3):
        expr_ramp(s, ch, 700, 2200, 96, 127)
        expr_ramp(s, ch, 2700, 3200, 127, 0)
    return s, 3700


def vitoria():
    s = Song()
    setup(s, 0, 56, rev=60, pan=52)
    setup(s, 1, 56, rev=60, pan=78, vol=98)
    setup(s, 2, 47, rev=60, vol=118)
    drums(s, 0)
    s.cc(0, DRUMS, 91, 60)
    tune = ((67, 0, 150), (72, 160, 150), (76, 320, 150), (79, 480, 330), (76, 830, 150), (79, 990, 1500))
    for key, at, dur in tune:
        s.note(at, 0, key, 116, dur)
        s.note(at + 8, 1, key - 12 if at >= 990 else key, 100, dur)
    for at, k, v in ((0, 43, 100), (480, 48, 110), (990, 48, 127)):
        s.note(at, 2, k, v, 400)
    s.note(990, DRUMS, 49, 116, 300)
    s.note(990, DRUMS, 36, 115, 200)
    for ch in (0, 1):
        expr_ramp(s, ch, 990, 2250, 127, 112)
        expr_ramp(s, ch, 2250, 2500, 112, 0)
    return s, 2900


def coro():
    s = Song()
    setup(s, 0, 52, rev=95)             # coro "aah"
    setup(s, 1, 46, rev=90, vol=96)     # harpa
    setup(s, 2, 14, rev=95, vol=88)     # sino tubular
    scale = (60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84)
    for i, k in enumerate(scale):
        s.note(i * 42, 1, k, 78 + i * 2, 1100)
    for k in (48, 55, 60, 64, 67, 72):
        s.note(450, 0, k, 102, 3000)
    s.note(450, 2, 72, 92, 1800)
    s.cc(0, 0, 11, 40)
    expr_ramp(s, 0, 450, 1150, 40, 127)
    expr_ramp(s, 0, 3050, 3700, 127, 0)
    return s, 4200


def tcharam():
    s = Song()
    setup(s, 0, 61, rev=60)
    setup(s, 1, 56, rev=60, vol=104)
    setup(s, 2, 47, rev=60, vol=120)
    drums(s, 0)
    s.cc(0, DRUMS, 91, 60)
    for k in (55, 59, 62, 67):
        s.note(0, 0, k, 108, 140)
    for k in (60, 64, 67, 72):
        s.note(200, 0, k, 124, 1450)
    s.note(200, 1, 76, 116, 1450)
    s.note(200, 2, 48, 125, 600)
    s.note(200, DRUMS, 49, 118, 300)
    s.note(200, DRUMS, 36, 118, 200)
    for ch in (0, 1):
        expr_ramp(s, ch, 1250, 1650, 127, 0)
    return s, 2100


def assobio():
    s = Song()
    setup(s, 0, 78, rev=30)
    s.bend_range(0, 0, 12)
    base = 86

    def glide(t0, t1, a, b, curve=1.0):
        s.ramp(t0, t1, lambda ms, x: s.bend(ms, 0, a + (b - a) * x ** curve, 12))

    s.bend(0, 0, -6, 12)
    s.note(5, 0, base, 112, 300)
    s.cc(0, 0, 11, 60)
    expr_ramp(s, 0, 5, 70, 60, 127)
    glide(5, 270, -6, 7, 1.6)
    expr_ramp(s, 0, 230, 305, 127, 30)
    s.bend(480, 0, -6, 12)
    s.note(520, 0, base, 112, 720)
    expr_ramp(s, 0, 520, 590, 60, 127)
    glide(520, 770, -6, 9, 1.5)
    glide(770, 1230, 9, -9, 0.8)
    expr_ramp(s, 0, 1090, 1240, 127, 20)
    s.bend(1300, 0, 0, 12)
    return s, 1650


def violino():
    s = Song()
    setup(s, 0, 40, rev=75)
    melody = ((69, 0, 380), (72, 400, 380), (71, 800, 380), (69, 1200, 380), (68, 1600, 580), (69, 2200, 1500))
    for key, at, dur in melody:
        s.note(at, 0, key, 98, dur)
        expr_ramp(s, 0, at, at + 140, 84, 118)

        def vib(ms, x, at=at):
            depth = min(1, max(0, (ms - at - 140) / 260)) * 0.24
            s.bend(ms, 0, depth * math.sin(2 * math.pi * 5.8 * (ms - at) / 1000))
        s.ramp(at, at + dur, vib)
    expr_ramp(s, 0, 3050, 3700, 118, 0)
    s.bend(3710, 0, 0)
    return s, 3950


def impacto():
    s = Song()
    setup(s, 0, 116, rev=85, vol=124)       # taiko
    setup(s, 1, 116, bank=8, rev=85)        # bumbo de concerto
    setup(s, 2, 47, rev=85, vol=124)        # tímpano
    setup(s, 3, 61, rev=90, vol=118)        # metais graves ("braam")
    drums(s, 0)
    s.cc(0, DRUMS, 91, 80)
    s.note(0, 0, 41, 127, 600)
    s.note(0, 1, 36, 127, 1500)
    s.note(0, 2, 41, 127, 900)
    for k in (36, 43, 48):
        s.note(0, 3, k, 127, 1300)
    expr_ramp(s, 3, 60, 1300, 127, 0)
    s.note(0, DRUMS, 49, 112, 300)
    s.note(0, DRUMS, 57, 100, 300)
    s.note(0, DRUMS, 36, 127, 200)
    return s, 3300


def sino():
    s = Song()
    setup(s, 0, 9, rev=55)       # glockenspiel
    setup(s, 1, 14, rev=60, vol=104)   # sino tubular
    for i, k in enumerate((84, 88, 91, 96)):
        s.note(i * 90, 0, k, 100 + i * 5, 700)
    s.note(270, 1, 72, 104, 1600)
    return s, 2300


def grilos(path):
    """Grilos de verdade cantam um tom quase puro (~4,5 kHz) em pulsos; síntese direta fica fiel."""
    rnd = np.random.default_rng(11)
    dur = 3.4
    n = int(SR * dur)
    out = np.zeros(n)
    t = np.arange(n) / SR

    def cricket(freq, period, start, gain, pulses=3):
        y = np.zeros(n)
        at = start
        while at < dur - 0.2:
            for p in range(pulses):
                p0 = at + p * 0.036
                i0, i1 = int(p0 * SR), int((p0 + 0.02) * SR)
                if i1 >= n:
                    break
                tt = np.arange(i1 - i0) / SR
                env = np.sin(np.pi * tt / 0.02) ** 1.5
                f = freq * (1 + rnd.normal(0, 0.004))
                y[i0:i1] += env * (np.sin(2 * np.pi * f * tt) + 0.08 * np.sin(4 * np.pi * f * tt))
            at += period * (1 + rnd.normal(0, 0.03))
        return gain * y

    out += cricket(4650, 0.42, 0.05, 1.0)
    out += cricket(4310, 0.51, 0.27, 0.55, pulses=4)
    out += cricket(4880, 0.36, 0.6, 0.28)
    # fundo noturno bem baixo
    noise = rnd.normal(0, 1, n)
    k = np.exp(-np.arange(2400) / 300.0)
    k /= k.sum()
    amb = np.convolve(noise, k, mode='same')
    out += 0.05 * amb / (np.abs(amb).max() + 1e-9)
    # um pouco de ambiente (reverberação simples)
    ir_len = int(0.5 * SR)
    ir = rnd.normal(0, 1, ir_len) * np.exp(-np.arange(ir_len) / (0.11 * SR))
    ir[0] = 0
    wet = np.convolve(out, ir)[:n]
    out = out + 0.06 * wet / (np.abs(wet).max() + 1e-9) * np.abs(out).max()
    fade = int(0.5 * SR)
    out[-fade:] *= np.linspace(1, 0, fade)
    out = 0.8 * out / np.abs(out).max()
    with wave.open(path, 'wb') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes((out * 32767).astype('<i2').tobytes())


SOUNDS = [
    ('aplausos', aplausos), ('badumtss', badumtss), ('trombone', trombone), ('grilos', None),
    ('suspense', suspense), ('rufar', rufar), ('vitoria', vitoria), ('tcharam', tcharam),
    ('coro', coro), ('assobio', assobio), ('violino', violino), ('impacto', impacto),
    ('sino', sino), ('palmas', palmas),
]


def run(cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True)


def finish(src, dst):
    """Corta silêncio, mono, volume padronizado (EBU R128) e Opus."""
    with tempfile.TemporaryDirectory() as tmp:
        trimmed = os.path.join(tmp, 't.wav')
        run(['ffmpeg', '-y', '-v', 'error', '-i', src, '-ac', '1', '-af',
             'silenceremove=start_periods=1:start_threshold=-58dB,areverse,'
             'silenceremove=start_periods=1:start_threshold=-62dB,areverse', trimmed])
        d = float(run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', trimmed]).stdout)
        fade = f'afade=t=in:d=0.004,afade=t=out:st={max(0, d - 0.12):.3f}:d=0.12'
        m = run(['ffmpeg', '-v', 'info', '-i', trimmed, '-af', fade + ',loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json',
                 '-f', 'null', '-'])
        js = json.loads(m.stderr[m.stderr.rindex('{'):m.stderr.rindex('}') + 1])
        ln = (f"loudnorm=I=-16:TP=-1.5:LRA=11:measured_I={js['input_i']}:measured_TP={js['input_tp']}:"
              f"measured_LRA={js['input_lra']}:measured_thresh={js['input_thresh']}:offset={js['target_offset']}:linear=true")
        run(['ffmpeg', '-y', '-v', 'error', '-i', trimmed, '-af', fade + ',' + ln, '-ar', '48000', '-ac', '1',
             '-c:a', 'libopus', '-b:a', '56k', '-vbr', 'on', '-compression_level', '10', '-application', 'audio',
             '-map_metadata', '-1', dst])
        return d


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    sf = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), '..', '..', 'web', 'sounds')
    os.makedirs(out, exist_ok=True)
    durs = {}
    with tempfile.TemporaryDirectory() as tmp:
        for name, fn in SOUNDS:
            wav = os.path.join(tmp, name + '.wav')
            if fn is None:
                grilos(wav)
            else:
                song, end = fn()
                mid = os.path.join(tmp, name + '.mid')
                song.save(mid, end)
                run(['fluidsynth', '-ni', '-q', '-g', '0.55', '-r', str(SR), '-R', '1', '-C', '0',
                     '-o', 'synth.reverb.room-size=0.62', '-o', 'synth.reverb.damp=0.35',
                     '-o', 'synth.reverb.width=0.9', '-o', 'synth.reverb.level=0.75',
                     '-F', wav, sf, mid])
            durs[name] = round(finish(wav, os.path.join(out, name + '.ogg')), 2)
            size = os.path.getsize(os.path.join(out, name + '.ogg'))
            print(f'{name:10s} {durs[name]:5.2f}s {size / 1024:6.1f} KB')
    print(json.dumps(durs))


if __name__ == '__main__':
    main()
