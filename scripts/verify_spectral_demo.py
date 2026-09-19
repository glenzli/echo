#!/usr/bin/env python3
"""Measure the exported calibration and verify immutable spectral-demo sources."""
import argparse
import array
import hashlib
import json
import math
from pathlib import Path
import sqlite3
import subprocess

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('fixture',type=Path)
parser.add_argument('--report',type=Path,required=True)
args=parser.parse_args()
manifest=json.loads((args.fixture/'sources.json').read_text())
for entry in [manifest['calibration'],*manifest['sources']]:
    assert hashlib.sha256(Path(entry['path']).read_bytes()).hexdigest()==entry['sha256'],entry['path']
def decode(path):
    data=subprocess.check_output(['ffmpeg','-v','error','-i',str(path),'-ar','48000','-ac','1','-f','f32le','-'])
    samples=array.array('f');samples.frombytes(data);return samples
def amplitude(samples,hz,start,end):
    first=round(start*48000);last=round(end*48000)
    real=sum(samples[i]*math.cos(2*math.pi*hz*i/48000) for i in range(first,last))
    imaginary=sum(samples[i]*math.sin(2*math.pi*hz*i/48000) for i in range(first,last))
    return 2*math.hypot(real,imaginary)/(last-first)
def db_ratio(after,before): return 20*math.log10(max(after,1e-12)/max(before,1e-12))
source=decode(args.fixture/'sources/Spectral repair — calibration.wav')
result=decode(args.fixture/'Spectral repair — result.wav')
assert len(result)==len(source)==8*48000
reduction=db_ratio(amplitude(result,1000,1,7),amplitude(source,1000,1,7))
retained=db_ratio(amplitude(result,3000,1,7),amplitude(source,3000,1,7))
assert -25<reduction<-23,reduction
assert abs(retained)<0.1,retained
facts={'calibrationFrames':len(result),'target1000HzChangeDb':reduction,'retained3000HzChangeDb':retained,'sourceHashesUnchanged':True}
voice_result=args.fixture/'旁白 — 修复后.wav'
if voice_result.exists():
    clean=decode(args.fixture/'sources/旁白 — 合成原声.wav')
    noisy=decode(args.fixture/'sources/旁白 — 啸叫练习.wav')
    repaired=decode(voice_result)
    assert len(repaired)==len(clean)==len(noisy)
    # Mono canonical playback upmixes at -3 dB per channel; ffmpeg's mono
    # comparison downmix restores that gain for this two-channel export.
    error_before=[noisy[i]-clean[i] for i in range(len(clean))]
    error_after=[repaired[i]-clean[i] for i in range(len(clean))]
    tone_before=amplitude(error_before,1800,4.3,6.7)
    tone_after=amplitude(error_after,1800,4.3,6.7)
    difference=max(abs(repaired[i]-noisy[i]) for i in range(48000,3*48000))
    facts.update(voiceFrames=len(repaired),injected1800HzChangeDb=db_ratio(tone_after,tone_before),outsideRepairMaximumError=difference)
    assert facts['injected1800HzChangeDb']<-18,facts
    assert difference<0.001,facts
connection=sqlite3.connect(args.fixture/'catalog.sqlite')
assert connection.execute('PRAGMA quick_check').fetchone()[0]=='ok'
assert not connection.execute('PRAGMA foreign_key_check').fetchall()
facts['catalogIntegrity']=True
args.report.write_text(json.dumps(facts,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(facts,ensure_ascii=False,indent=2))
