#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Two-stage SQL detection pipeline for astgrep rules (Plan 2):
1) Extract SQL statements from Java and MyBatis XML sources, normalize placeholders
2) Run existing SQL rules with astgrep on the extracted SQL snippets
3) Map findings back to original files and output a consolidated JSON report

Usage examples:
  python scripts/extract_sql_and_analyze.py \
      --targets src/ test/sql-1 \
      --config test/sql-1/sql_rules_either_sc.yaml \
      --astgrep target/debug/astgrep

Outputs:
  - tmp/sql-extract/*.sql           Extracted SQL snippets
  - tmp/sql-extract/mapping.json    Mapping from snippet files to original source info
  - tmp/sql-extract/report.json     Raw astgrep JSON findings on extracted SQL
  - tmp/sql-extract/final_report.json  Findings mapped back to original sources
"""

import argparse
import json
import os
import re
import sys
import subprocess
from pathlib import Path
from typing import Dict, List, Tuple, Any

# ---------------------------- Helpers ----------------------------

def ensure_tmp_dir(tmp_dir: Path):
    tmp_dir.mkdir(parents=True, exist_ok=True)


def write_text(path: Path, content: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding='utf-8')


def unescape_java_string(s: str) -> str:
    # Basic unescape for Java-like string literals
    s = s.replace('\\"', '"').replace("\\'", "'")
    s = s.replace('\\n', '\n').replace('\\r', '\r').replace('\\t', '\t')
    s = s.replace('\\\n', '\n')
    return s


def collapse_ws(s: str) -> str:
    return re.sub(r"\s+", " ", s).strip()


def normalize_sql(sql: str) -> str:
    sql = collapse_ws(sql)
    if not sql.endswith(';'):
        sql += ';'
    return sql


def strip_namespace(tag: str) -> str:
    return tag.split('}', 1)[-1]


# ------------------------ Java extraction ------------------------

JAVA_METHOD_PATTERN = re.compile(
    r"\b(?:prepareStatement|executeQuery|executeUpdate|createNativeQuery)\s*\((.*?)\)",
    re.DOTALL,
)
JAVA_STRING_LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')
JAVA_ANNOT_SELECT = re.compile(r'@(?:[\w\.]+\.)?Select\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)')


def extract_sql_from_java(java_code: str) -> List[Tuple[str, int, str]]:
    """
    Return list of tuples: (sql_text, approx_start_line, context_desc)
    context_desc e.g., 'prepareStatement', '@Select'
    """
    results: List[Tuple[str, int, str]] = []

    # 1) @Select("...")
    for m in JAVA_ANNOT_SELECT.finditer(java_code):
        start_line = java_code.count('\n', 0, m.start()) + 1
        raw = m.group(1)
        sql = unescape_java_string(raw)
        sql = normalize_sql(sql)
        results.append((sql, start_line, '@Select'))

    # 2) Method calls: prepareStatement(...), executeQuery(...), etc.
    for m in JAVA_METHOD_PATTERN.finditer(java_code):
        approx_line = java_code.count('\n', 0, m.start()) + 1
        arg_expr = m.group(1)
        context = 'call'
        # Attempt to reconstruct string expression: concatenate literals, replace others with ' ? '
        parts = []
        last_end = 0
        had_literal = False
        for sm in JAVA_STRING_LITERAL.finditer(arg_expr):
            # non-literal between last_end and sm.start()
            gap = arg_expr[last_end:sm.start()]
            if gap.strip():
                parts.append(' ? ')
            lit = unescape_java_string(sm.group(1))
            parts.append(lit)
            had_literal = True
            last_end = sm.end()
        # tail gap
        tail = arg_expr[last_end:]
        if tail.strip():
            parts.append(' ? ')
        if not had_literal:
            continue  # skip if no obvious SQL string present
        sql = normalize_sql(''.join(parts))
        results.append((sql, approx_line, context))

    return results


# ------------------------- XML extraction ------------------------

XML_TARGET_TAGS = {"select", "sql", "insert", "update", "delete"}


def extract_sql_from_xml(xml_path: Path) -> List[Tuple[str, int, str]]:
    results: List[Tuple[str, int, str]] = []
    text = xml_path.read_text(encoding='utf-8', errors='ignore')

    # Try XML parser first
    try:
        import xml.etree.ElementTree as ET
        it = ET.iterparse(str(xml_path), events=("start", "end"))
        # Build mapping from element to start position is not available; we will approximate via regex search
        root = None
        for event, elem in it:
            if event == 'start' and root is None:
                root = elem
            if event == 'end':
                tag = strip_namespace(elem.tag)
                if tag in XML_TARGET_TAGS:
                    # Flatten inner text
                    sql_text = ''.join(elem.itertext())
                    sql_text = sql_text or ''
                    # Normalize MyBatis placeholders
                    sql_text = re.sub(r"#\{[^}]+\}", "1", sql_text)  # value placeholders
                    sql_text = re.sub(r"\$\{[^}]+\}", "T0", sql_text)  # identifier-like placeholders
                    sql_text = normalize_sql(sql_text)
                    # Approx line using a simple search
                    idx = text.find(sql_text[:40].replace(';', '').strip())
                    approx_line = text.count('\n', 0, idx) + 1 if idx >= 0 else 1
                    results.append((sql_text, approx_line, f'<{tag}>'))
                # Clear to save memory on big files
                elem.clear()
    except Exception:
        # Fallback: regex-based extraction for <select>...</select> etc.
        for tag in XML_TARGET_TAGS:
            pattern = re.compile(rf'<{tag}[^>]*>([\s\S]*?)</{tag}>', re.IGNORECASE)
            for m in pattern.finditer(text):
                approx_line = text.count('\n', 0, m.start()) + 1
                sql_text = m.group(1)
                sql_text = re.sub(r"<!--.*?-->", " ", sql_text, flags=re.DOTALL)
                sql_text = re.sub(r"#\{[^}]+\}", "1", sql_text)
                sql_text = re.sub(r"\$\{[^}]+\}", "T0", sql_text)
                sql_text = normalize_sql(sql_text)
                results.append((sql_text, approx_line, f'<{tag}>'))

    return results


# -------------------------- Main routine -------------------------

def _extract_balanced_braces(text: str, start_idx: int):
    depth = 0
    in_str = False
    esc = False
    for i in range(start_idx, len(text)):
        ch = text[i]
        if in_str:
            if esc:
                esc = False
            elif ch == '\\':
                esc = True
            elif ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
            elif ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth == 0:
                    return text[start_idx:i+1]
    return None


def _parse_astgrep_mixed_output(stdout: str) -> Dict[str, Any]:
    # First, try direct JSON
    try:
        return json.loads(stdout)
    except Exception:
        pass
    # Look for a JSON object that starts with {"findings"
    anchor = '{"findings"'
    idx = stdout.find(anchor)
    if idx != -1:
        block = _extract_balanced_braces(stdout, idx)
        if block:
            return json.loads(block)
    # As a fallback, scan for any '{' where the next ~200 chars contain "findings"
    for m in re.finditer(r'\{', stdout):
        i = m.start()
        if '"findings"' in stdout[i:i+200]:
            block = _extract_balanced_braces(stdout, i)
            if block:
                return json.loads(block)
    raise ValueError('Could not locate JSON block in astgrep output')


def run_astgrep(astgrep_bin: Path, config: Path, extract_dir: Path) -> Dict[str, Any]:
    # Prefer quiet mode to suppress INFO/DEBUG logs mixing with JSON. Fallback if unsupported.
    cmd_quiet = [str(astgrep_bin), '-q', 'analyze', '--language', 'sql', '--config', str(config), str(extract_dir)]
    proc = subprocess.run(cmd_quiet, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0 and ('Found argument' in proc.stderr and '-q' in proc.stderr):
        # Retry without -q if the binary doesn't support it
        cmd = [str(astgrep_bin), 'analyze', '--language', 'sql', '--config', str(config), str(extract_dir)]
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if proc.returncode != 0:
        print(proc.stdout)
        print(proc.stderr, file=sys.stderr)
        raise SystemExit(f"astgrep failed with exit code {proc.returncode}")
    try:
        return _parse_astgrep_mixed_output(proc.stdout)
    except Exception as e:
        raise SystemExit(f"Failed to parse astgrep JSON output: {e}\nOutput was:\n{proc.stdout}")


def main():
    ap = argparse.ArgumentParser(description='Extract SQL from Java/MyBatis XML and analyze with astgrep SQL rules')
    ap.add_argument('--targets', nargs='*', default=['.'], help='Files or directories to scan (default: .)')
    ap.add_argument('--config', required=True, help='SQL rule YAML (the one you already use for .sql files)')
    ap.add_argument('--astgrep', default='target/debug/astgrep', help='Path to astgrep binary (default: target/debug/astgrep)')
    ap.add_argument('--tmp-dir', default='tmp/sql-extract', help='Directory to write extracted SQL and reports')
    ap.add_argument('--ext', nargs='*', default=['.java', '.xml'], help='File extensions to include (default: .java .xml)')
    args = ap.parse_args()

    targets = [Path(t) for t in args.targets]
    config = Path(args.config)
    astgrep_bin = Path(args.astgrep)
    extract_dir = Path(args.tmp_dir)
    ensure_tmp_dir(extract_dir)

    # Remove previous extracted_*.sql files to avoid stale results
    for old in extract_dir.glob('extracted_*.sql'):
        try:
            old.unlink()
        except Exception:
            pass

    mapping: Dict[str, Dict[str, Any]] = {}
    snippet_count = 0

    # Collect files
    files: List[Path] = []
    for t in targets:
        if t.is_file():
            files.append(t)
        else:
            for root, _, fnames in os.walk(t):
                for fn in fnames:
                    if any(fn.lower().endswith(ext) for ext in args.ext):
                        files.append(Path(root) / fn)

    # Extract
    for f in files:
        try:
            if f.suffix.lower() == '.java':
                code = f.read_text(encoding='utf-8', errors='ignore')
                extracted = extract_sql_from_java(code)
            elif f.suffix.lower() == '.xml':
                extracted = extract_sql_from_xml(f)
            else:
                continue
        except Exception as e:
            print(f"[WARN] Failed to process {f}: {e}", file=sys.stderr)
            continue

        for sql_text, approx_line, ctx in extracted:
            snippet_count += 1
            out_path = extract_dir / f"extracted_{snippet_count:05d}.sql"
            header = f"-- SOURCE: {f} | line~{approx_line} | ctx={ctx}\n"
            write_text(out_path, header + sql_text + "\n")
            mapping[str(out_path)] = {
                'source_file': str(f),
                'approx_start_line': approx_line,
                'context': ctx,
                'normalized_sql': sql_text,
            }

    # Save mapping
    write_text(extract_dir / 'mapping.json', json.dumps(mapping, ensure_ascii=False, indent=2))

    if snippet_count == 0:
        print('No SQL snippets extracted. Nothing to analyze.')
        return

    # Run astgrep on the extracted snippets
    result = run_astgrep(astgrep_bin, config, extract_dir)
    write_text(extract_dir / 'report.json', json.dumps(result, ensure_ascii=False, indent=2))

    # Map findings back to original files
    findings = result.get('findings', [])
    final_findings: List[Dict[str, Any]] = []
    for fd in findings:
        loc = fd.get('location', {})
        fpath = loc.get('file')
        if not fpath:
            continue
        m = mapping.get(fpath)
        if not m:
            # sometimes file path may be relative; try to resolve
            abs_path = str((Path.cwd() / fpath).resolve())
            m = mapping.get(abs_path)
        if not m:
            # cannot map back; keep as-is
            final_findings.append(fd)
            continue
        # Build mapped location
        approx_line = m['approx_start_line']
        mapped_loc = {
            'file': m['source_file'],
            'start_line': approx_line,
            'end_line': approx_line,
            'start_column': 1,
            'end_column': 120,
        }
        new_fd = dict(fd)
        new_fd['location'] = mapped_loc
        new_fd['extracted_from'] = fpath
        new_fd['context'] = m['context']
        new_fd['normalized_sql'] = m['normalized_sql']
        final_findings.append(new_fd)

    final_report = {
        'findings': final_findings,
        'summary': result.get('summary', {}),
        'extraction': {
            'snippets': snippet_count,
            'sources_scanned': len(files),
            'tmp_dir': str(extract_dir),
        }
    }
    write_text(extract_dir / 'final_report.json', json.dumps(final_report, ensure_ascii=False, indent=2))

    # Print brief summary
    print(json.dumps({
        'snippets': snippet_count,
        'files_scanned': len(files),
        'findings': len(final_findings),
        'report': str(extract_dir / 'final_report.json')
    }, ensure_ascii=False))


if __name__ == '__main__':
    main()

