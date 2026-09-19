#!/usr/bin/env python3
"""Create attributed, deterministic spectral repair fixtures without changing input audio."""
import argparse
import array
import hashlib
import json
import math
from pathlib import Path
import shutil
import sys
import wave

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('output',type=Path)
parser.add_argument('--voice',type=Path,help='Existing Infer Runtime PCM16 mono narration fixture')
args=parser.parse_args()
sources=args.output/'sources'
sources.mkdir(parents=True,exist_ok=True)
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def save(path,rate,channels,samples):
    if path.exists(): raise SystemExit(f'Refusing to replace existing fixture: {path}')
    data=array.array('h',samples)
    if sys.byteorder!='little': data.byteswap()
    with wave.open(str(path),'wb') as out:
        out.setnchannels(channels);out.setsampwidth(2);out.setframerate(rate);out.writeframes(data.tobytes())
calibration=sources/'Spectral repair — calibration.wav'
values=[]
for frame in range(48000*8):
    time=frame/48000
    sample=round(32767*(0.16*math.sin(2*math.pi*1000*time)+0.04*math.sin(2*math.pi*3000*time)))
    values.extend([sample,sample])
save(calibration,48000,2,values)
manifest={'description':'Synthetic calibration; 1000 Hz at 0.16 and 3000 Hz at 0.04, 8 seconds stereo.',
          'calibration':{'path':str(calibration),'sha256':digest(calibration)},'sources':[]}
if args.voice:
    with wave.open(str(args.voice),'rb') as inp:
        if inp.getsampwidth()!=2 or inp.getnchannels()!=1: raise SystemExit('Voice must be PCM16 mono')
        rate=inp.getframerate();data=array.array('h',inp.readframes(inp.getnframes()))
        if sys.byteorder!='little': data.byteswap()
    clean=sources/'旁白 — 合成原声.wav'
    if clean.exists(): raise SystemExit(f'Refusing to replace {clean}')
    shutil.copy2(args.voice,clean)
    damaged=[]
    for index,sample in enumerate(data):
        time=index/rate
        envelope=max(0,min(1,(time-4)/0.02,(7-time)/0.02))
        interference=0.06*envelope*math.sin(2*math.pi*1800*time)
        damaged.append(max(-32768,min(32767,round(sample+32767*interference))))
    noisy=sources/'旁白 — 啸叫练习.wav'
    save(noisy,rate,1,damaged)
    manifest['sources']=[{'path':str(clean),'sha256':digest(clean),'parent':str(args.voice),'parentSha256':digest(args.voice),
        'origin':'Existing Infer Runtime synthetic narration; generation provenance in the multitrack fixture sources.json.'},
        {'path':str(noisy),'sha256':digest(noisy),'parentSha256':digest(clean),
        'addedInterference':{'frequencyHz':1800,'amplitude':0.06,'startSeconds':4,'endSeconds':7,'edgeSeconds':0.02}}]
(args.output/'sources.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(args.output)
