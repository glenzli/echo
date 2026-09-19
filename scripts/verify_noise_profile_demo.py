#!/usr/bin/env python3
"""Quantify real exported noise reduction and preserve source evidence."""
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
for entry in manifest['sources']:
    assert hashlib.sha256(Path(entry['path']).read_bytes()).hexdigest()==entry['sha256']
def decode(path):
    value=array.array('f'); value.frombytes(subprocess.check_output(['ffmpeg','-v','error','-i',str(path),'-ar','48000','-ac','1','-f','f32le','-'])); return value
def rms(values,start,end):
    first,last=round(start*48000),round(end*48000)
    return math.sqrt(sum(value*value for value in values[first:last])/(last-first))
def db(after,before): return 20*math.log10(max(after,1e-15)/max(before,1e-15))
def tone(values,hz,start,end):
    first,last=round(start*48000),round(end*48000)
    re=sum(values[i]*math.cos(2*math.pi*hz*i/48000) for i in range(first,last))
    im=sum(values[i]*math.sin(2*math.pi*hz*i/48000) for i in range(first,last))
    return 2*math.hypot(re,im)/(last-first)
source=decode(args.fixture/'sources/Noise reduction — calibration.wav')
result=decode(args.fixture/'Noise reduction — result.wav')
assert len(source)==len(result)==6*48000
facts={'sourceHashesUnchanged':True,'calibrationNoiseChangeDb':db(rms(result,4.5,5.8),rms(source,4.5,5.8)),
       'retained1000HzChangeDb':db(tone(result,1000,2.5,3.5),tone(source,1000,2.5,3.5))}
assert facts['calibrationNoiseChangeDb'] < -10, facts
assert abs(facts['retained1000HzChangeDb'])<.5, facts
clean=decode(args.fixture/'sources/旁白 — 合成基准.wav')
noisy=decode(args.fixture/'sources/旁白 — 底噪练习.wav')
repaired=decode(args.fixture/'旁白 — 降噪后.wav')
assert len(clean)==len(noisy)==len(repaired)
before=[value-clean[i] for i,value in enumerate(noisy)]
after=[value-clean[i] for i,value in enumerate(repaired)]
facts.update(voiceFrames=len(clean),voiceNoiseChangeDb=db(rms(repaired,.2,1),rms(noisy,.2,1)),
             voiceErrorChangeDb=db(rms(after,1.3,13.2),rms(before,1.3,13.2)))
assert facts['voiceNoiseChangeDb'] < -7, facts
assert facts['voiceErrorChangeDb'] < 0, facts
connection=sqlite3.connect(args.fixture/'catalog.sqlite')
assert connection.execute('PRAGMA quick_check').fetchone()[0]=='ok'
assert not connection.execute('PRAGMA foreign_key_check').fetchall()
facts['catalogIntegrity']=True
args.report.write_text(json.dumps(facts,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(facts,ensure_ascii=False,indent=2))
