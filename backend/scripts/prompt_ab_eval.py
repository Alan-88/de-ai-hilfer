#!/usr/bin/env python3
from __future__ import annotations

import argparse
from datetime import datetime
from pathlib import Path
import sys
from prompt_ab_eval_core import (
    build_blind_outputs,
    export_backup,
    parse_bool,
    read_cases,
    run_variant,
    start_server,
    stop_server,
    summarize_variant,
    write_blind_review_jsonl,
    write_json,
    write_rating_template,
)


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    backend_dir = repo_root / "backend"
    default_cases = repo_root / "backend" / "eval" / "cases" / "pilot_20.txt"
    default_prompt = (repo_root / "legacy_reference" / "backend" / "config.yaml").resolve()

    parser = argparse.ArgumentParser(
        description="A/B evaluate prompt variants with DB snapshot reset and blind output generation."
    )
    parser.add_argument("--backend-dir", type=Path, default=backend_dir)
    parser.add_argument("--cases", type=Path, default=default_cases)
    parser.add_argument("--prompt-a", type=Path, default=default_prompt)
    parser.add_argument("--prompt-b", type=Path, default=default_prompt)
    parser.add_argument("--mode", choices=["readonly", "accumulative"], default="readonly")
    parser.add_argument("--quality-mode", choices=["default", "pro"], default="default")
    parser.add_argument("--force-refresh", type=parse_bool, default=True)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--request-timeout", type=float, default=90.0)
    parser.add_argument("--startup-timeout", type=float, default=90.0)
    parser.add_argument("--bootstrap-port", type=int, default=38210)
    parser.add_argument("--port-a", type=int, default=38211)
    parser.add_argument("--port-b", type=int, default=38212)
    parser.add_argument("--seed", type=int, default=20260316)
    parser.add_argument("--output-dir", type=Path, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.output_dir is None:
        stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        args.output_dir = args.backend_dir / "logs" / f"prompt_ab_eval_{stamp}"
    args.output_dir.mkdir(parents=True, exist_ok=True)

    args.cases = args.cases.resolve()
    args.prompt_a = args.prompt_a.resolve()
    args.prompt_b = args.prompt_b.resolve()
    args.backend_dir = args.backend_dir.resolve()

    if not args.cases.exists():
        raise FileNotFoundError(f"cases file not found: {args.cases}")
    if not args.prompt_a.exists():
        raise FileNotFoundError(f"prompt A file not found: {args.prompt_a}")
    if not args.prompt_b.exists():
        raise FileNotFoundError(f"prompt B file not found: {args.prompt_b}")

    cases = read_cases(args.cases, args.limit)
    print(f"Loaded {len(cases)} cases from {args.cases}")
    print(f"Output dir: {args.output_dir}")

    bootstrap = start_server(
        backend_dir=args.backend_dir,
        prompt_config_path=args.prompt_a,
        port=args.bootstrap_port,
        label="bootstrap",
        output_dir=args.output_dir,
        startup_timeout=args.startup_timeout,
    )
    try:
        baseline_backup = export_backup(bootstrap.base_url, timeout=args.request_timeout)
    finally:
        stop_server(bootstrap)

    (args.output_dir / "baseline_backup.json").write_bytes(baseline_backup)

    variant_a_rows = run_variant(
        label="A",
        prompt_config_path=args.prompt_a,
        port=args.port_a,
        cases=cases,
        args=args,
        baseline_backup=baseline_backup,
        output_dir=args.output_dir,
    )
    variant_b_rows = run_variant(
        label="B",
        prompt_config_path=args.prompt_b,
        port=args.port_b,
        cases=cases,
        args=args,
        baseline_backup=baseline_backup,
        output_dir=args.output_dir,
    )

    blind_rows, mapping_rows, diff_rows = build_blind_outputs(
        variant_a_rows=variant_a_rows,
        variant_b_rows=variant_b_rows,
        seed=args.seed,
    )

    write_json(args.output_dir / "raw_variant_a.json", variant_a_rows)
    write_json(args.output_dir / "raw_variant_b.json", variant_b_rows)
    write_json(args.output_dir / "blind_mapping.json", mapping_rows)
    write_json(args.output_dir / "diff_summary.json", diff_rows)
    write_blind_review_jsonl(args.output_dir / "blind_review.jsonl", blind_rows)
    write_rating_template(args.output_dir / "rating_template.csv", blind_rows)

    summary = {
        "created_at": datetime.now().isoformat(),
        "mode": args.mode,
        "quality_mode": args.quality_mode,
        "force_refresh": args.force_refresh,
        "cases_file": str(args.cases),
        "prompt_a": str(args.prompt_a),
        "prompt_b": str(args.prompt_b),
        "seed": args.seed,
        "variant_a": summarize_variant("A", variant_a_rows),
        "variant_b": summarize_variant("B", variant_b_rows),
    }
    write_json(args.output_dir / "summary.json", summary)

    print("A/B evaluation complete.")
    print(f"Summary: {args.output_dir / 'summary.json'}")
    print(f"Blind set: {args.output_dir / 'blind_review.jsonl'}")
    print(f"Score sheet: {args.output_dir / 'rating_template.csv'}")
    print(f"Mapping (keep hidden): {args.output_dir / 'blind_mapping.json'}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        raise SystemExit(130)
