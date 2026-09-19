#!/usr/bin/env python3
"""Create isolated, explicitly synthetic noise-learning fixtures without replacing originals."""
import argparse
import array
import hashlib
import json
import math
from pathlib import Path
import random
import subprocess
import wave

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('output', type=Path)
parser.add_argument('--voice', type=Path, required=True)
args = parser.parse_args()
sources = args.output / 'sources'
sources.mkdir(parents=True, exist_ok=True)
rate = 48000

def write(name, values):
    path = sources / name
    if path.exists():
        raise FileExistsError(path)
    samples = array.array('h', (round(max(-1, min(1, value)) * 32767) for value in values))
    with wave.open(str(path), 'wb') as output:
        output.setnchannels(1); output.setsampwidth(2); output.setframerate(rate); output.writeframes(samples.tobytes())
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}

rng = random.Random(971)
calibration = [rng.uniform(-.03,.03) + .02*math.sin(2*math.pi*180*i/rate)
               + (.25*math.sin(2*math.pi*1000*i/rate) if 2*rate<=i<4*rate else 0) for i in range(6*rate)]
records = [write('Noise reduction — calibration.wav', calibration)]
decoded = array.array('f')
decoded.frombytes(subprocess.check_output(['ffmpeg','-v','error','-i',str(args.voice),'-ar',str(rate),'-ac','1','-f','f32le','-']))
clean = [0.0]*round(1.2*rate) + list(decoded) + [0.0]*round(1.2*rate)
records.append(write('旁白 — 合成基准.wav',clean))
noisy = [value+rng.uniform(-.014,.014)+.012*math.sin(2*math.pi*180*i/rate) for i,value in enumerate(clean)]
records.append(write('旁白 — 底噪练习.wav',noisy))
manifest = {'purpose':'Synthetic steady-noise debug fixtures; never field recordings',
            'voiceSource':str(args.voice),'voiceSourceSha256':hashlib.sha256(args.voice.read_bytes()).hexdigest(),
            'seed':971,'sampleRate':rate,'voicePaddingSeconds':1.2,'sources':records}
(args.output/'sources.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(manifest,ensure_ascii=False,indent=2))
