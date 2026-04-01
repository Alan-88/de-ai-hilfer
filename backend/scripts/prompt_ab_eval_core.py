#!/usr/bin/env python3
from __future__ import annotations

import csv
import difflib
import json
import os
import random
import signal
import subprocess
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib import error, request


@dataclass
class ServerHandle:
    process: subprocess.Popen[str]
    log_path: Path
    log_fp: Any
    base_url: str


def parse_bool(value: str) -> bool:
    normalized = value.strip().lower()
    if normalized in {"1", "true", "yes", "y", "on"}:
        return True
    if normalized in {"0", "false", "no", "n", "off"}:
        return False
    raise ValueError(f"invalid boolean value: {value}")


def read_cases(path: Path, limit: int | None) -> list[str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    cases: list[str] = []
    for line in lines:
        query = line.strip()
        if not query or query.startswith("#"):
            continue
        cases.append(query)
    if limit is not None:
        cases = cases[:limit]
    if not cases:
        raise ValueError(f"no cases found in {path}")
    return cases


def request_bytes(
    method: str,
    url: str,
    data: bytes | None = None,
    headers: dict[str, str] | None = None,
    timeout: float = 30.0,
) -> bytes:
    req = request.Request(url=url, data=data, method=method, headers=headers or {})
    try:
        with request.urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {url} failed: HTTP {exc.code} - {body}") from exc
    except error.URLError as exc:
        raise RuntimeError(f"{method} {url} failed: {exc}") from exc


def request_json(
    method: str,
    url: str,
    payload: dict[str, Any] | None = None,
    timeout: float = 30.0,
) -> dict[str, Any]:
    data = None
    headers: dict[str, str] = {}
    if payload is not None:
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        headers["Content-Type"] = "application/json"
    body = request_bytes(method=method, url=url, data=data, headers=headers, timeout=timeout)
    return json.loads(body.decode("utf-8"))


def build_multipart(field_name: str, filename: str, content: bytes) -> tuple[bytes, str]:
    boundary = f"----de-ai-hilfer-{uuid.uuid4().hex}"
    crlf = b"\r\n"
    chunks = [
        f"--{boundary}".encode("utf-8"),
        f'Content-Disposition: form-data; name="{field_name}"; filename="{filename}"'.encode(
            "utf-8"
        ),
        b"Content-Type: application/json",
        b"",
        content,
        f"--{boundary}--".encode("utf-8"),
        b"",
    ]
    return crlf.join(chunks), f"multipart/form-data; boundary={boundary}"


def wait_for_health(base_url: str, timeout_sec: float) -> None:
    deadline = time.time() + timeout_sec
    health_url = f"{base_url}/health"
    last_error = "unknown"
    while time.time() < deadline:
        try:
            body = request_bytes("GET", health_url, timeout=2.0).decode("utf-8", errors="replace")
            if body.strip() == "OK":
                return
            last_error = f"unexpected response: {body!r}"
        except Exception as exc:  # noqa: BLE001
            last_error = str(exc)
        time.sleep(0.8)
    raise TimeoutError(f"server health check timed out: {last_error}")


def read_log_tail(log_path: Path, lines: int = 40) -> str:
    if not log_path.exists():
        return "<log file missing>"
    all_lines = log_path.read_text(encoding="utf-8", errors="replace").splitlines()
    return "\n".join(all_lines[-lines:])


def start_server(
    backend_dir: Path,
    prompt_config_path: Path,
    port: int,
    label: str,
    output_dir: Path,
    startup_timeout: float,
) -> ServerHandle:
    log_path = output_dir / f"server_{label}.log"
    log_fp = log_path.open("w", encoding="utf-8")

    env = os.environ.copy()
    env["SERVER_PORT"] = str(port)
    env["SERVER_HOST"] = "127.0.0.1"
    env["PROMPT_CONFIG_PATH"] = str(prompt_config_path)

    process = subprocess.Popen(
        ["cargo", "run", "--bin", "server"],
        cwd=str(backend_dir),
        env=env,
        stdout=log_fp,
        stderr=subprocess.STDOUT,
        text=True,
    )
    base_url = f"http://127.0.0.1:{port}"
    handle = ServerHandle(process=process, log_path=log_path, log_fp=log_fp, base_url=base_url)
    try:
        wait_for_health(base_url, startup_timeout)
    except Exception:
        stop_server(handle)
        tail = read_log_tail(log_path)
        raise RuntimeError(
            f"failed to start server for {label} on port {port}; log tail:\n{tail}"
        ) from None
    return handle


def stop_server(handle: ServerHandle) -> None:
    process = handle.process
    if process.poll() is None:
        try:
            process.send_signal(signal.SIGINT)
            process.wait(timeout=10)
        except Exception:  # noqa: BLE001
            try:
                process.terminate()
                process.wait(timeout=5)
            except Exception:  # noqa: BLE001
                process.kill()
    try:
        handle.log_fp.flush()
    except Exception:  # noqa: BLE001
        pass
    try:
        handle.log_fp.close()
    except Exception:  # noqa: BLE001
        pass


def export_backup(base_url: str, timeout: float) -> bytes:
    return request_bytes("GET", f"{base_url}/api/v1/database/export", timeout=timeout)


def import_backup(base_url: str, backup_bytes: bytes, timeout: float) -> dict[str, Any]:
    body, content_type = build_multipart("backup_file", "backup.json", backup_bytes)
    response = request_bytes(
        "POST",
        f"{base_url}/api/v1/database/import",
        data=body,
        headers={"Content-Type": content_type},
        timeout=timeout,
    )
    return json.loads(response.decode("utf-8"))


def run_analyze_case(
    base_url: str,
    query: str,
    quality_mode: str,
    force_refresh: bool,
    timeout: float,
) -> dict[str, Any]:
    payload = {
        "query_text": query,
        "quality_mode": quality_mode,
        "force_refresh": force_refresh,
    }
    return request_json(
        method="POST",
        url=f"{base_url}/api/v1/analyze",
        payload=payload,
        timeout=timeout,
    )


def run_variant(
    label: str,
    prompt_config_path: Path,
    port: int,
    cases: list[str],
    args: Any,
    baseline_backup: bytes,
    output_dir: Path,
) -> list[dict[str, Any]]:
    handle = start_server(
        backend_dir=args.backend_dir,
        prompt_config_path=prompt_config_path,
        port=port,
        label=label,
        output_dir=output_dir,
        startup_timeout=args.startup_timeout,
    )
    rows: list[dict[str, Any]] = []
    try:
        import_backup(handle.base_url, baseline_backup, timeout=args.request_timeout)
        for idx, query in enumerate(cases, start=1):
            if args.mode == "readonly":
                import_backup(handle.base_url, baseline_backup, timeout=args.request_timeout)
            started = time.perf_counter()
            try:
                response = run_analyze_case(
                    base_url=handle.base_url,
                    query=query,
                    quality_mode=args.quality_mode,
                    force_refresh=args.force_refresh,
                    timeout=args.request_timeout,
                )
                elapsed_ms = int((time.perf_counter() - started) * 1000)
                rows.append(
                    {
                        "case_id": idx,
                        "query": query,
                        "status": "ok",
                        "latency_ms": elapsed_ms,
                        "entry_id": response.get("entry_id"),
                        "source": response.get("source"),
                        "model": response.get("model"),
                        "analysis_markdown": response.get("analysis_markdown", ""),
                        "quality_mode": response.get("quality_mode"),
                        "error": None,
                    }
                )
            except Exception as exc:  # noqa: BLE001
                elapsed_ms = int((time.perf_counter() - started) * 1000)
                rows.append(
                    {
                        "case_id": idx,
                        "query": query,
                        "status": "error",
                        "latency_ms": elapsed_ms,
                        "entry_id": None,
                        "source": None,
                        "model": None,
                        "analysis_markdown": "",
                        "quality_mode": None,
                        "error": str(exc),
                    }
                )
            print(f"[{label}] {idx}/{len(cases)} {query}")
        import_backup(handle.base_url, baseline_backup, timeout=args.request_timeout)
    finally:
        stop_server(handle)
    return rows


def summarize_variant(name: str, rows: list[dict[str, Any]]) -> dict[str, Any]:
    ok_rows = [row for row in rows if row["status"] == "ok"]
    error_rows = [row for row in rows if row["status"] != "ok"]
    if ok_rows:
        avg_latency = sum(row["latency_ms"] for row in ok_rows) / len(ok_rows)
        avg_length = (
            sum(len((row.get("analysis_markdown") or "").strip()) for row in ok_rows) / len(ok_rows)
        )
    else:
        avg_latency = 0.0
        avg_length = 0.0
    return {
        "variant": name,
        "total_cases": len(rows),
        "ok_cases": len(ok_rows),
        "error_cases": len(error_rows),
        "avg_latency_ms": round(avg_latency, 2),
        "avg_markdown_chars": round(avg_length, 2),
    }


def build_blind_outputs(
    variant_a_rows: list[dict[str, Any]],
    variant_b_rows: list[dict[str, Any]],
    seed: int,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    if len(variant_a_rows) != len(variant_b_rows):
        raise ValueError("A/B case counts do not match")
    rng = random.Random(seed)
    blind_rows: list[dict[str, Any]] = []
    mapping_rows: list[dict[str, Any]] = []
    diff_rows: list[dict[str, Any]] = []
    for left, right in zip(variant_a_rows, variant_b_rows):
        if left["case_id"] != right["case_id"] or left["query"] != right["query"]:
            raise ValueError("A/B case alignment mismatch")
        similarity = difflib.SequenceMatcher(
            a=(left.get("analysis_markdown") or ""),
            b=(right.get("analysis_markdown") or ""),
        ).ratio()
        shuffled = [("A", left), ("B", right)]
        rng.shuffle(shuffled)
        x_variant, x_row = shuffled[0]
        y_variant, y_row = shuffled[1]
        blind_rows.append(
            {
                "case_id": left["case_id"],
                "query": left["query"],
                "X": {
                    "status": x_row["status"],
                    "latency_ms": x_row["latency_ms"],
                    "source": x_row["source"],
                    "model": x_row["model"],
                    "analysis_markdown": x_row["analysis_markdown"],
                    "error": x_row["error"],
                },
                "Y": {
                    "status": y_row["status"],
                    "latency_ms": y_row["latency_ms"],
                    "source": y_row["source"],
                    "model": y_row["model"],
                    "analysis_markdown": y_row["analysis_markdown"],
                    "error": y_row["error"],
                },
            }
        )
        mapping_rows.append(
            {
                "case_id": left["case_id"],
                "query": left["query"],
                "X": x_variant,
                "Y": y_variant,
            }
        )
        diff_rows.append(
            {
                "case_id": left["case_id"],
                "query": left["query"],
                "similarity_ab": round(similarity, 4),
                "a_status": left["status"],
                "b_status": right["status"],
                "a_latency_ms": left["latency_ms"],
                "b_latency_ms": right["latency_ms"],
            }
        )
    return blind_rows, mapping_rows, diff_rows


def write_json(path: Path, payload: Any) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def write_blind_review_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as fp:
        for row in rows:
            fp.write(json.dumps(row, ensure_ascii=False))
            fp.write("\n")


def write_rating_template(path: Path, blind_rows: list[dict[str, Any]]) -> None:
    headers = [
        "case_id",
        "query",
        "winner(X|Y|tie)",
        "accuracy_score_1_5",
        "readability_score_1_5",
        "learning_value_score_1_5",
        "conciseness_score_1_5",
        "notes",
    ]
    with path.open("w", encoding="utf-8", newline="") as fp:
        writer = csv.writer(fp)
        writer.writerow(headers)
        for row in blind_rows:
            writer.writerow([row["case_id"], row["query"], "", "", "", "", "", ""])
