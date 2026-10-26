#!/usr/bin/env python3
"""Capture the verified public report with natural synthetic narration.

Optional authoring dependencies (not required by make demo):
  python -m pip install --target .tools/video-tools playwright imageio-ffmpeg edge-tts
The neural voice uses Microsoft's online Edge speech service. No API key is used.
Stage directions are omitted from speech; captions use actual TTS word timings.
"""
from __future__ import annotations

import argparse
import asyncio
from datetime import datetime, timezone
import hashlib
import html
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import textwrap
import time
import urllib.parse
import urllib.request
import wave

ROOT = next(p for p in Path(__file__).resolve().parents if (p / 'Cargo.toml').is_file() and (p / '.git').exists())
sys.path.insert(0, str(ROOT / '.tools/video-tools'))
DURATION = 120
SAMPLE_RATE = 44100
VOICE = 'en-US-AndrewMultilingualNeural'
RATE = '+6%'


def run(command: list[str], capture: bool = False) -> subprocess.CompletedProcess:
    return subprocess.run(command, check=True, capture_output=capture, text=capture,
                          creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical(text: str) -> str:
    return ''.join(c.casefold() for c in html.unescape(text) if c.isalnum())


def read_narration(path: Path) -> list[dict]:
    text = path.read_text(encoding='utf-8-sig')
    headings = list(re.finditer(r'^\*\*(\d+):(\d+)[–-](\d+):(\d+)[^\n]*\*\*\s*$', text, re.M))
    segments = []
    for index, heading in enumerate(headings):
        start = 60 * int(heading[1]) + int(heading[2])
        end = 60 * int(heading[3]) + int(heading[4])
        stop = headings[index + 1].start() if index + 1 < len(headings) else len(text)
        body = text[heading.end():stop].strip()
        markers = list(re.finditer(r'\[pause(?:\s+([0-9]+(?:\.[0-9]+)?)s)?\]',body))
        starts = [0]+[marker.end() for marker in markers]
        stops = [marker.start() for marker in markers]+[len(body)]
        chunks = [' '.join(body[a:b].split()) for a,b in zip(starts,stops)]
        pauses = [float(marker[1]) if marker[1] else 0.4 for marker in markers]
        if not all(0.15 <= seconds <= 1.5 for seconds in pauses):
            raise ValueError('Explicit pauses must be between 0.15 and 1.5 seconds')
        if not all(chunks) or end <= start or any('[' in c or ']' in c for c in chunks):
            raise ValueError('Each timed segment needs speech; only [pause] or [pause 0.5s] directions are supported')
        segments.append({'start':start,'end':end,'chunks':chunks,'pauses':pauses,'text':' '.join(chunks)})
    if not segments or segments[0]['start'] != 0 or segments[-1]['end'] != DURATION:
        raise ValueError('Narration must cover 0:00 through 2:00')
    if any(a['end'] != b['start'] for a, b in zip(segments, segments[1:])):
        raise ValueError('Narration intervals must be contiguous')
    return segments


def verify_narration_facts(path: Path,result: dict) -> dict:
    match=re.search(r'<!-- evidence:\s*(\{[^\n]+\})\s*-->',path.read_text(encoding='utf-8-sig'))
    if not match:raise ValueError('Narration must declare its exact numeric evidence claims')
    claimed=json.loads(match[1])
    actual={'checks':len(result['exercised_checks']),
            'calls':result['leaky']['observation_count']+result['fixed']['observation_count'],
            'candidates':len(result['candidate_txids']),
            'leaky':sum(f['leaked'] for f in result['leaky']['findings']),
            'fixed':sum(f['leaked'] for f in result['fixed']['findings'])}
    if claimed!=actual:raise ValueError(f'Narrated numbers do not match live evidence: claimed {claimed}, actual {actual}')
    return actual


def get_bytes(url: str) -> bytes:
    request = urllib.request.Request(url, headers={'User-Agent':'Darkroom-live-evidence-recorder','Cache-Control':'no-cache'})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def verify_result(result: dict, expected: set[str]) -> None:
    if result.get('passed') is not True or set(result.get('expected_leaks', [])) != expected:
        raise ValueError('Public report does not pass the requested exact findings')
    for name in ('leaky', 'fixed'):
        run_result = result[name]
        if run_result.get('validated') is not True:
            raise ValueError(f'{name} evidence has not been validated')
        findings = {f['id']:f for f in run_result['findings']}
        if not all(findings[key]['exercised'] for key in expected):
            raise ValueError(f'{name} skipped an expected check')
        actual = {key for key, finding in findings.items() if finding['leaked']}
        if actual != (expected if name == 'leaky' else set()):
            raise ValueError('Actual findings disagree with the requested narration scope')


def snapshot_live(base_url: str, directory: Path, expected: set[str]) -> tuple[dict, dict, str]:
    parsed = urllib.parse.urlsplit(base_url)
    if parsed.scheme != 'https' or not parsed.netloc:
        raise ValueError('The primary report source must be a public HTTPS URL')
    base_url = base_url.rstrip('/') + '/'
    token = str(time.time_ns())
    source = directory / 'source'
    source.mkdir()
    hashes = {}
    for name in ['index.html','result.json','leaky-observations.jsonl','fixed-observations.jsonl',
                 'leaky-app.log','fixed-app.log','leaky-wallet.json','fixed-wallet.json']:
        data = get_bytes(base_url + name + '?darkroom=' + token)
        (source / name).write_bytes(data)
        hashes[name] = hashlib.sha256(data).hexdigest()
    result = json.loads((source / 'result.json').read_text(encoding='utf-8'))
    verify_result(result, expected)
    provenance = result.get('provenance', {})
    ci_url = provenance.get('ci_run_url', '')
    match = re.fullmatch(r'https://github\.com/([\w.-]+/[\w.-]+)/actions/runs/(\d+)', ci_url)
    if not match:
        raise ValueError('The public report does not identify its producing CI run')
    ci = json.loads(get_bytes(f'https://api.github.com/repos/{match[1]}/actions/runs/{match[2]}'))
    if ci.get('status') != 'completed' or ci.get('conclusion') != 'success':
        raise ValueError('The producing CI run has not completed successfully')
    if ci.get('head_sha') != provenance.get('commit') or ci.get('head_branch') != 'main':
        raise ValueError('Public evidence does not match the successful main-branch CI commit')
    manifest = {'public_url':base_url,'downloaded_at_utc':datetime.now(timezone.utc).isoformat(),
                'hashes':hashes,'ci_run_url':ci_url,'ci_commit':ci['head_sha'],
                'ci_conclusion':ci['conclusion'],'expected_checks':sorted(expected)}
    (source / 'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(f"Verified public {','.join(sorted(expected))} evidence and successful CI {match[2]}", flush=True)
    return result, manifest, base_url + '?darkroom=' + token


def caption_groups(text: str, events: list[dict], chunk_offset: float) -> list[dict]:
    # Align metadata to the original words so punctuation and literal file/RPC names
    # remain verbatim even when the speech service splits one token into boundaries.
    positions = [(c.casefold(), index) for index, c in enumerate(text) if c.isalnum()]
    normalized = ''.join(c for c, _ in positions)
    cursor = 0
    mapped = []
    for event in events:
        token = canonical(event['text'])
        if not token or normalized[cursor:cursor+len(token)] != token:
            raise ValueError(f"Word-boundary metadata disagrees with narration near {event['text']!r}")
        start_index = positions[cursor][1]
        cursor += len(token)
        end_index = positions[cursor-1][1] + 1
        while end_index < len(text) and not text[end_index].isalnum() and not text[end_index].isspace():
            end_index += 1
        mapped.append({'start':chunk_offset+event['offset']/10_000_000,
                       'end':chunk_offset+(event['offset']+event['duration'])/10_000_000,
                       'first':start_index,'last':end_index})
    if cursor != len(normalized):
        raise ValueError('Speech timing metadata omitted narration words')
    groups = []
    pending = []
    for index, word in enumerate(mapped):
        pending.append(word)
        first, last = pending[0], pending[-1]
        content = text[first['first']:last['last']]
        glued = index+1 < len(mapped) and not any(c.isspace() for c in text[last['last']:mapped[index+1]['first']])
        at_end = index+1 == len(mapped)
        duration = last['end']-first['start']
        short_tail = len(mapped)-index-1 <= 2 and len(text[first['first']:]) <= 78
        boundary = (len(pending) >= 6 and duration >= 1.2 and not short_tail) or len(content) >= 64 or duration >= 2.8 or content.endswith(('.', '!', '?'))
        if at_end or (not glued and boundary):
            groups.append({'start':first['start'],'end':last['end'],'text':content})
            pending = []
    if canonical(' '.join(group['text'] for group in groups)) != canonical(text):
        raise ValueError('Caption wording differs from the narration')
    return groups


async def speech_chunk(text: str, voice: str, output: Path) -> list[dict]:
    import edge_tts
    request = {'text':text,'voice':voice,'rate':RATE,'boundary':'WordBoundary'}
    key = hashlib.sha256(json.dumps(request,sort_keys=True).encode()).hexdigest()
    cache = ROOT / '.tools/video-speech-cache'
    cache.mkdir(exist_ok=True)
    cached_audio, cached_metadata = cache / f'{key}.mp3', cache / f'{key}.json'
    if cached_audio.is_file() and cached_metadata.is_file():
        try:
            saved = json.loads(cached_metadata.read_text(encoding='utf-8'))
            if saved['request'] == request and saved['events'] and saved['audio_sha256'] == sha(cached_audio):
                output.write_bytes(cached_audio.read_bytes())
                output.with_suffix('.json').write_text(json.dumps(saved,indent=2),encoding='utf-8')
                return saved['events']
        except (OSError,ValueError,KeyError):
            pass
    for attempt in range(3):
        events = []
        try:
            communicator = edge_tts.Communicate(text,voice,rate=request['rate'],boundary='WordBoundary',
                                                connect_timeout=15,receive_timeout=45)
            with output.open('wb') as audio:
                async for item in communicator.stream():
                    if item['type']=='audio':audio.write(item['data'])
                    elif item['type']=='WordBoundary':events.append(item)
            if not events or not output.stat().st_size:
                raise RuntimeError('Speech service returned no usable audio or word timings')
            caption_groups(text,events,0)  # Reject omitted or substituted words before caching.
            saved={'request':request,'events':events,'audio_sha256':sha(output)}
            metadata=json.dumps(saved,indent=2)
            output.with_suffix('.json').write_text(metadata,encoding='utf-8')
            cached_audio.write_bytes(output.read_bytes())
            cached_metadata.write_text(metadata,encoding='utf-8')
            return events
        except Exception as error:
            if attempt==2:raise
            print(f'Neural speech request retry {attempt+1}/2 after {type(error).__name__}',flush=True)
            await asyncio.sleep(2**(attempt+1))
    raise RuntimeError('Speech request did not finish')


async def synthesize_audio(segments: list[dict], directory: Path, ffmpeg: str, voice: str) -> tuple[Path, list[dict], list[dict]]:
    import edge_tts
    voices = await asyncio.wait_for(edge_tts.list_voices(), timeout=30)
    if not any(v['ShortName'] == voice and 'Neural' in voice for v in voices):
        raise ValueError('The requested neural voice is not currently available')
    output_frames = bytearray(DURATION * SAMPLE_RATE * 2)
    captions, all_words = [], []
    for index, segment in enumerate(segments):
        frames = bytearray()
        groups, words = [], []
        for part, text in enumerate(segment['chunks']):
            if part:
                frames.extend(b'\0' * round(segment['pauses'][part-1] * SAMPLE_RATE) * 2)
            offset = len(frames) / (SAMPLE_RATE * 2)
            mp3 = directory / f'voice-{index}-{part}.mp3'
            events = await speech_chunk(text,voice,mp3)
            decoded = mp3.with_suffix('.wav')
            run([ffmpeg,'-v','error','-y','-i',str(mp3),'-ar',str(SAMPLE_RATE),'-ac','1','-c:a','pcm_s16le',str(decoded)])
            with wave.open(str(decoded),'rb') as wav:
                frames.extend(wav.readframes(wav.getnframes()))
            groups.extend(caption_groups(text,events,offset))
            words.extend({'text':e['text'],'start':offset+e['offset']/10_000_000,
                          'end':offset+(e['offset']+e['duration'])/10_000_000} for e in events)
        seconds = len(frames)/(SAMPLE_RATE*2)
        available = segment['end']-segment['start']-0.25
        speed = max(1.0,seconds/available)
        if speed > 1.10:
            raise ValueError(f'Segment {index+1} needs a shorter script ({seconds:.2f}s for {available:.2f}s); refusing rushed narration')
        if speed > 1:
            source = directory / f'segment-{index}.wav'
            with wave.open(str(source),'wb') as wav:
                wav.setparams((1,2,SAMPLE_RATE,0,'NONE','not compressed')); wav.writeframes(frames)
            adjusted = directory / f'segment-{index}-fit.wav'
            run([ffmpeg,'-v','error','-y','-i',str(source),'-af',f'atempo={speed:.8f}',str(adjusted)])
            with wave.open(str(adjusted),'rb') as wav:
                frames = bytearray(wav.readframes(wav.getnframes()))
        start_frame = round(segment['start']*SAMPLE_RATE)*2
        if start_frame+len(frames) > round(segment['end']*SAMPLE_RATE)*2:
            raise ValueError('Narration overlaps its next visual segment')
        output_frames[start_frame:start_frame+len(frames)] = frames
        for item in groups:
            captions.append({'start':segment['start']+item['start']/speed,
                             'end':segment['start']+item['end']/speed,'text':item['text']})
        for item in words:
            all_words.append({'start':segment['start']+item['start']/speed,
                              'end':segment['start']+item['end']/speed,'text':item['text']})
        print(f"Neural narration segment {index+1}: {seconds:.2f}s; timing ratio {speed:.3f}",flush=True)
    plain = directory / 'narration-plain.wav'
    with wave.open(str(plain),'wb') as wav:
        wav.setparams((1,2,SAMPLE_RATE,0,'NONE','not compressed')); wav.writeframes(output_frames)
    audio = directory / 'narration.wav'
    run([ffmpeg,'-v','error','-y','-i',str(plain),'-af','loudnorm=I=-16:TP=-1.5:LRA=11',
         '-ar',str(SAMPLE_RATE),'-ac','1','-t',str(DURATION),'-c:a','pcm_s16le',str(audio)])
    if canonical(' '.join(c['text'] for c in captions)) != canonical(' '.join(s['text'] for s in segments)):
        raise ValueError('Final caption transcript is not word-for-word narration')
    (directory/'word-timings.json').write_text(json.dumps(all_words,indent=2),encoding='utf-8')
    return audio,captions,all_words


def render_vtt(captions: list[dict]) -> str:
    def stamp(seconds: float) -> str:
        milliseconds = round(seconds*1000)
        return f'{milliseconds//3600000:02}:{milliseconds//60000%60:02}:{milliseconds//1000%60:02}.{milliseconds%1000:03}'
    chunks = ['WEBVTT','']
    for index,cue in enumerate(captions):
        end = min(cue['end']+0.1, captions[index+1]['start'] if index+1<len(captions) else DURATION)
        if cue['start'] >= end:
            raise ValueError('Caption has an invalid time interval')
        text = '\n'.join(textwrap.wrap(cue['text'],width=44,break_long_words=False,break_on_hyphens=False))
        chunks.extend([f"{stamp(cue['start'])} --> {stamp(end)}",html.escape(text,quote=False),''])
    return '\n'.join(chunks)


def phrase_interval(words: list[dict],phrase: str,start: float,end: float) -> dict:
    wanted=canonical(phrase)
    for index,word in enumerate(words):
        if not start<=word['start']<end:continue
        found=''
        for last in words[index:]:
            found+=canonical(last['text'])
            if found==wanted and last['end']<=end:
                return {'phrase':phrase,'start':word['start'],'end':last['end'],'visible_slot':[start,end]}
            if not wanted.startswith(found):break
    raise ValueError(f'Claim {phrase!r} is not spoken inside its visible {start}–{end}s slot')


def verify_claim_timings(words: list[dict]) -> list[dict]:
    claims=[('A1',13,31),('GetBlockRange',13,31),('A3',61,84),('app.log',61,84),
            ('A2',84,94),('SendTransaction',84,94),('unsigned',84,94),('refuses',84,94),
            ('Darkside',94,109),('consensus',94,109),('successful CI run',109,116),
            ('raw traces',109,116),('linked here',109,116)]
    return [phrase_interval(words,*claim) for claim in claims]


def record_page(url: str, directory: Path, edge: Path, manifest: dict, has_a2: bool) -> Path:
    from playwright.sync_api import sync_playwright
    shots = [(0,'header.hero',102),(13,'#a1',24),(31,'#a1',24),(48,'#a1 .comparison',40),
             (61,'#a3',24),(75,'#a3 .comparison',40),(84,'#a2' if has_a2 else '.unexercised',24),
             (94,'.methodology',24),(109,'.run-command',300),(116,'header.hero',102)]
    with sync_playwright() as p:
        browser = p.chromium.launch(executable_path=str(edge),headless=True)
        context = browser.new_context(viewport={'width':1440,'height':810},
            record_video_dir=str(directory),record_video_size={'width':1440,'height':810},
            device_scale_factor=1,reduced_motion='reduce')
        page = context.new_page()
        response = page.goto(url,wait_until='load')
        if not response or hashlib.sha256(response.body()).hexdigest() != manifest['hashes']['index.html']:
            raise ValueError('Live report changed between evidence snapshot and browser capture')
        page.locator('#a1 .state').first.wait_for()
        if not page.locator('.status.good').count():
            raise ValueError('Browser did not render a passing report')
        if has_a2 and not page.locator('#a2 .comparison').count():
            raise ValueError('The live page does not contain the exercised A2 comparison')
        page.mouse.move(1430,800)
        recording = page.video
        started = time.monotonic()
        for timestamp,selector,offset in shots:
            while (remaining:=timestamp-(time.monotonic()-started))>0:
                page.wait_for_timeout(min(remaining,1)*1000)
            page.locator(selector).first.evaluate('(el,offset)=>window.scrollTo(0,el.getBoundingClientRect().top+window.scrollY-offset)',offset)
            page.screenshot(path=str(directory/f'shot-{timestamp:03}.png'))
            if timestamp==109:
                for href in [manifest['ci_run_url'],'leaky-observations.jsonl','fixed-observations.jsonl']:
                    box=page.locator(f'a[href="{href}"]').last.bounding_box()
                    if not box or box['y']<0 or box['y']+box['height']>810:
                        raise ValueError(f'Narrated closing link is outside the viewport: {href}')
            print(f'Recorded live cue {timestamp:03}s: {selector}',flush=True)
        while (remaining:=DURATION+0.5-(time.monotonic()-started))>0:
            page.wait_for_timeout(min(remaining,1)*1000)
        context.close(); path=Path(recording.path()); browser.close()
    return path


def measure_lead_in(ffmpeg: str,path: Path) -> dict:
    # The first deliberate visual cut is hero -> A1 at cue 13. Measure its actual
    # encoded frame time; page creation/loading happens before our cue clock.
    output=run([ffmpeg,'-v','error','-i',str(path),'-an','-vf',
        "select='between(t,12,25)*gte(scene,0)',metadata=print:file=-",'-f','null','-'],capture=True)
    candidates=[];at=None
    for line in output.stdout.splitlines():
        match=re.search(r'pts_time:([0-9.]+)',line)
        if match:at=float(match[1])
        if line.startswith('lavfi.scene_score=') and at is not None:
            candidates.append((float(line.split('=')[1]),at))
    if not candidates:raise ValueError('No frames available to measure recording lead-in')
    score,cut=max(candidates)
    lead=cut-13
    if score<0.02 or not 0<=lead<=12:
        raise ValueError(f'Could not reliably locate the first hero-to-A1 cut: {cut}s, score {score}')
    return {'trim_seconds':lead,'first_cut_raw_seconds':cut,'first_cut_output_seconds':13,'scene_score':score}


def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--live-url',default='https://willy264.github.io/zcash/')
    parser.add_argument('--script',type=Path,default=ROOT/'docs/video-script.md')
    parser.add_argument('--output',type=Path,default=ROOT/'docs/demo.mp4')
    parser.add_argument('--voice',default=VOICE)
    parser.add_argument('--expected-checks',default='A1,A2,A3')
    parser.add_argument('--edge',type=Path,default=Path(r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'))
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    expected=set(args.expected_checks.split(','))
    if expected not in ({'A1','A3'},{'A1','A2','A3'}):
        raise ValueError('Supported scopes are the verified two-check or three-check report')
    take=ROOT/'.tools/video-takes'/str(time.time_ns());take.mkdir(parents=True)
    result,manifest,url=snapshot_live(args.live_url,take,expected)
    segments=read_narration(args.script)
    numeric_facts=verify_narration_facts(args.script,result)
    if args.check:
        print(f'Ready: live {sorted(expected)}, successful CI, {len(segments)} timed narration segments.');return
    import imageio_ffmpeg
    ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
    narration,captions,words=asyncio.run(synthesize_audio(segments,take,ffmpeg,args.voice))
    claim_timings=verify_claim_timings(words) if 'A2' in expected else []
    if not args.edge.is_file():raise FileNotFoundError('Microsoft Edge executable is unavailable')
    recording=record_page(url,take,args.edge,manifest,'A2' in expected)
    lead_in=measure_lead_in(ffmpeg,recording)
    print(f"Trimming measured browser lead-in: {lead_in['trim_seconds']:.3f}s",flush=True)
    # The primary evidence must still be the same successful public report.
    current=get_bytes(args.live_url.rstrip('/')+'/result.json?darkroom='+str(time.time_ns()))
    if hashlib.sha256(current).hexdigest()!=manifest['hashes']['result.json']:
        raise ValueError('Public evidence changed during recording; refusing to publish mixed versions')
    encoded=take/'demo.mp4'
    command=[ffmpeg,'-v','error','-y','-ss',str(lead_in['trim_seconds']),'-i',str(recording),'-i',str(narration),'-t',str(DURATION),'-map','0:v:0','-map','1:a:0']
    command+=['-vf','scale=1280:720,format=yuv420p','-r','24','-c:v','libx264','-preset','medium','-crf','25']
    command+=['-c:a','aac','-b:a','96k','-movflags','+faststart',str(encoded)]
    run(command)
    if encoded.stat().st_size>15*1024*1024:raise ValueError('Video exceeds the 15 MiB artifact budget')
    run([ffmpeg,'-v','error','-i',str(encoded),'-f','null','-'])
    vtt=render_vtt(captions)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    os.replace(encoded,args.output)
    args.output.with_suffix('.vtt').write_text(vtt,encoding='utf-8',newline='\n')
    proof={'duration_seconds':DURATION,'narration':'Synthetic neural speech; not a recording of a human narrator',
           'voice':args.voice,'rate':RATE,'speech_service':'Microsoft Edge online speech via edge-tts',
           'numeric_facts':numeric_facts,
           'captions':'Verbatim narration aligned to actual TTS WordBoundary timings',
           'caption_cues':len(captions),'word_boundaries':len(words),'word_timings':words,
           'on_screen_claim_timings':claim_timings,
           'report_sha256':manifest['hashes']['index.html'],'result_sha256':manifest['hashes']['result.json'],
           'live_source':manifest,'source_commit':result['provenance']['commit'],
           'leaks_exercised':sorted(expected),'transparent_fallback':
           'Unsigned transparent test submission after local insufficient scanned funds; no funds moved or consensus claim' if 'A2'in expected else 'not exercised',
           'capture':'Updated public report in Microsoft Edge; same original viewing sequence',
           'recording_lead_in':lead_in,'video_sha256':sha(args.output),
           'captions_sha256':sha(args.output.with_suffix('.vtt')),'narration_source_sha256':sha(args.script)}
    args.output.with_suffix('.json').write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(f'Saved {args.output}: {args.output.stat().st_size/1024/1024:.2f} MiB, {len(captions)} aligned caption cues.',flush=True)


if __name__=='__main__':main()
