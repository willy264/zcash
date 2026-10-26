#!/usr/bin/env python3
"""Publish an isolated review path while preserving every main Pages file.

Used only by the manually dispatched replay-branch job. No report is rebuilt or
edited here: both inputs are original successful CI artifacts.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import time
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CONFIG = json.loads((ROOT / '.github/preview-base.json').read_text(encoding='utf-8'))


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main_is_pinned() -> None:
    actual = subprocess.check_output([
        'gh', 'api', f"repos/{CONFIG['repository']}/branches/main", '--jq', '.commit.sha'
    ], text=True).strip()
    if actual != CONFIG['main_commit']:
        raise ValueError('Main advanced; refusing to deploy a stale main report')


def files(directory: Path) -> dict[str, bytes]:
    result = {}
    for path in directory.rglob('*'):
        if path.is_symlink():
            raise ValueError(f'Symlinks are not allowed in Pages artifacts: {path}')
        if path.is_file():
            result[path.relative_to(directory).as_posix()] = path.read_bytes()
    if not result:
        raise ValueError(f'Empty Pages artifact: {directory}')
    return result


def archive_files(archive: Path) -> dict[str, bytes]:
    result = {}
    with tarfile.open(archive) as source:
        for member in source.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or '..' in path.parts or '\\' in member.name or ':' in member.name:
                raise ValueError('Unsafe archive path')
            if member.isdir():
                continue
            if not member.isfile() or member.size > 100 * 1024 * 1024:
                raise ValueError('Only bounded regular files are allowed in Pages artifacts')
            name = path.as_posix()
            if name in result:
                raise ValueError('Duplicate archive path')
            stream = source.extractfile(member)
            if stream is None:
                raise ValueError('Missing archive file data')
            result[name] = stream.read()
    if not result:
        raise ValueError('Empty main artifact')
    return result


def validate_report(source: dict[str, bytes], commit: str) -> dict:
    report = json.loads(source['result.json'])
    checks = {'A1', 'A3', 'A2'}
    if not report['passed'] or report['provenance']['commit'] != commit:
        raise ValueError('Report did not pass or has the wrong source commit')
    if set(report['exercised_checks']) != checks:
        raise ValueError('Report scope must remain the same three checks')
    for profile in ('leaky', 'fixed'):
        findings = report[profile]['findings']
        if {f['id'] for f in findings if f['exercised']} != checks:
            raise ValueError(f'{profile} skipped a check')
        if {f['id'] for f in findings if f['leaked']} != (checks if profile == 'leaky' else set()):
            raise ValueError(f'{profile} findings differ from the submission scope')
    return report


def verify_live(expected: dict[str, str]) -> None:
    def check(item: tuple[str, str]) -> None:
        name, expected_hash = item
        url = CONFIG['site_url'] + urllib.parse.quote(name, safe='/')
        request = urllib.request.Request(url, headers={'Cache-Control': 'no-cache'})
        with urllib.request.urlopen(request, timeout=45) as response:
            actual = digest(response.read())
        if actual != expected_hash:
            raise ValueError(f'Published bytes do not match the CI artifact: {name}')
    with ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(check, expected.items()))


def prepare(args: argparse.Namespace) -> None:
    main_is_pinned()
    baseline = archive_files(args.main_archive)
    branch = files(args.branch_report)
    main_report = validate_report(baseline, CONFIG['main_commit'])
    if not main_report['provenance']['ci_run_url'].endswith('/' + str(CONFIG['main_run'])):
        raise ValueError('Main artifact was not produced by the pinned successful run')
    validate_report(branch, args.branch_sha)
    prefix = CONFIG['preview_path'] + '/'
    if any(name.startswith(prefix) for name in baseline):
        raise ValueError('The pinned main artifact already occupies the review path')
    main_hashes = {name: digest(data) for name, data in baseline.items()}
    verify_live(main_hashes)
    combined = dict(baseline)
    combined.update({prefix + name: data for name, data in branch.items()})
    manifest = {'main_commit': CONFIG['main_commit'], 'main_run': CONFIG['main_run'],
                'branch_commit': args.branch_sha, 'main_files': main_hashes,
                'preview_files': {prefix + name: digest(data) for name, data in branch.items()}}
    combined[prefix + 'preview-manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    # Refuse to reuse a directory: no deletion, overwriting, or stale files.
    args.output.mkdir(parents=True, exist_ok=False)
    for name, data in combined.items():
        target = args.output / name
        if not target.resolve().is_relative_to(args.output.resolve()):
            raise ValueError('Output path escaped the staging directory')
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    for name, data in baseline.items():
        if (args.output / name).read_bytes() != data:
            raise ValueError(f'Main artifact changed during composition: {name}')
    main_is_pinned()
    print(f'Prepared preview: {len(baseline)} main files preserved byte-for-byte; '
          f'{len(branch)} branch files under {prefix}')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    build = sub.add_parser('prepare')
    build.add_argument('--main-archive', type=Path, required=True)
    build.add_argument('--branch-report', type=Path, required=True)
    build.add_argument('--branch-sha', required=True)
    build.add_argument('--output', type=Path, required=True)
    verify = sub.add_parser('verify')
    verify.add_argument('--output', type=Path, required=True)
    sub.add_parser('check-main')
    args = parser.parse_args()
    if args.command == 'prepare':
        prepare(args)
    elif args.command == 'verify':
        manifest = json.loads((args.output / CONFIG['preview_path'] / 'preview-manifest.json').read_bytes())
        for attempt in range(6):
            try:
                verify_live(manifest['main_files'] | manifest['preview_files'])
                break
            except (OSError, ValueError):
                if attempt == 5:
                    raise
                print('Waiting for the new Pages files to reach the serving edge...', flush=True)
                time.sleep(5)
        main_is_pinned()
        print('Published root and preview bytes match their original CI artifacts.')
    else:
        main_is_pinned()


if __name__ == '__main__':
    main()
