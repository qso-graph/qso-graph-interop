#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Prove independent fixtures reject the review's paired Status Rx/Tx swap.

Run with uv run --no-project python scripts/check-field-order-mutation.py.
All mutations occur in a temporary copy; the source tree is never changed.
"""
import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--log-dir', type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    logs = args.log_dir or root / 'target' / 'review-mutations'
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='qg-field-order-') as temporary:
        work = Path(temporary) / 'work'
        shutil.copytree(root, work, ignore=shutil.ignore_patterns('.git', 'target', 'corpus', 'artifacts'))
        codec = work / 'crates/qso-graph-interop/src/wsjtx/codec.rs'
        original = codec.read_text()
        # These are the exact two replacements from /tmp/qg-interop-review/mutate.py.
        for pattern in [
            'rx_df_hz: r.u32()?,\n            tx_df_hz: r.u32()?,',
            'w.u32(v.rx_df_hz)?;\n            w.u32(v.tx_df_hz)?;',
        ]:
            if original.count(pattern) != 1:
                raise SystemExit('expected exactly one site for each half of the paired mutation')
        mutant = original.replace(
            'rx_df_hz: r.u32()?,\n            tx_df_hz: r.u32()?,',
            'tx_df_hz: r.u32()?,\n            rx_df_hz: r.u32()?,', 1
        ).replace(
            'w.u32(v.rx_df_hz)?;\n            w.u32(v.tx_df_hz)?;',
            'w.u32(v.tx_df_hz)?;\n            w.u32(v.rx_df_hz)?;', 1
        )
        if mutant == original:
            raise SystemExit('mutation did not apply')
        codec.write_text(mutant)
        result = subprocess.run(
            ['cargo', 'test', '--workspace', '--all-features', '--locked', '--no-fail-fast'],
            cwd=work, capture_output=True, check=False,
        )
        output = result.stdout + result.stderr
        (logs / 'field-order.log').write_bytes(output)
        print(f'paired Status Rx/Tx swap: cargo test exit={result.returncode}')
        if result.returncode == 0:
            raise SystemExit('FAIL: mutation survived the complete workspace suite')
        if b'test wsjtx_status ... FAILED' not in output:
            raise SystemExit('FAIL: expected independent Status fixture failure was absent; inspect log')
        print('PASS: independent Status fixture killed the paired encoder/decoder mutation')


if __name__ == '__main__':
    main()
