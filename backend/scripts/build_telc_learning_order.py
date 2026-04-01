#!/Users/server/.pyenv/versions/miniforge3-25.3.1-0/bin/python

import argparse
import json
import os
import re
from dataclasses import dataclass
from pathlib import Path

import psycopg
from pypdf import PdfReader


ARTICLE_PREFIXES = ("der/die/das", "der/die", "der", "die", "das")
ARABIC_RE = re.compile(r"[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF]")
SKIP_PREFIXES = (
    "Einfach gut!",
    "Einfach besser!",
    "© telc",
    "Artikel Deutsch",
    "Bsp. -",
    "Wortschatzliste",
)


@dataclass(frozen=True)
class SourceSpec:
    path: Path
    cefr_level: str
    cefr_rank: int
    book_code: str
    requires_non_latin: bool = False


DEFAULT_SPECS = [
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_a1_1.pdf"), "A1", 1, "telc_einfach_gut_a1_1"),
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_a1_2.pdf"), "A1", 1, "telc_einfach_gut_a1_2"),
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_a2_1.pdf"), "A2", 2, "telc_einfach_gut_a2_1"),
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_a2_2.pdf"), "A2", 2, "telc_einfach_gut_a2_2"),
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_b1_1.pdf"), "B1", 3, "telc_einfach_gut_b1_1"),
    SourceSpec(Path("/tmp/telc_vocab_sources/telc_b1_2.pdf"), "B1", 3, "telc_einfach_gut_b1_2"),
    SourceSpec(
        Path("/tmp/telc_vocab_sources/telc_b2_400.pdf"),
        "B2",
        4,
        "telc_einfach_besser_400",
        requires_non_latin=True,
    ),
    SourceSpec(
        Path("/tmp/telc_vocab_sources/telc_b2_500.pdf"),
        "B2",
        4,
        "telc_einfach_besser_500",
        requires_non_latin=True,
    ),
]


def build_headword_map(conn) -> dict[str, list[str]]:
    with conn.cursor() as cur:
        cur.execute("SELECT headword FROM dictionary_raw")
        rows = cur.fetchall()
    mapping = {}
    for (headword,) in rows:
        key = headword.lower()
        mapping.setdefault(key, [])
        if headword not in mapping[key]:
            mapping[key].append(headword)
    return mapping


def is_noise(line: str) -> bool:
    stripped = line.strip()
    return (
        not stripped
        or stripped.isdigit()
        or any(stripped.startswith(prefix) for prefix in SKIP_PREFIXES)
    )


def clean_candidate(candidate: str) -> str:
    value = candidate.strip().strip(".,;:!?")
    value = re.sub(r"\s+", " ", value)
    return value


def extract_lesson_number(line: str) -> int | None:
    match = re.search(r"Wortschatz zu Lektion\s+(\d+)", line)
    if match:
        return int(match.group(1))
    return None


def candidate_prefixes(line: str, allow_backoff: bool = True) -> list[str]:
    stripped = line.strip()
    stripped = re.sub(r"\s+", " ", stripped)
    stripped = re.sub(r"\([^)]*\)", "", stripped).strip()
    tokens = stripped.split(" ")
    candidates: list[str] = []
    article_led = False

    for article in ARTICLE_PREFIXES:
        if stripped.startswith(f"{article} "):
            article_led = True
            tail = stripped[len(article) + 1 :]
            tail_tokens = tail.split(" ")
            first_upper_index = next(
                (index for index, token in enumerate(tail_tokens) if token[:1].isupper()),
                None,
            )

            min_length = 1
            if (
                len(tail_tokens) > 1
                and tail_tokens[0][:1].isupper()
                and tail_tokens[1][:1].islower()
            ):
                min_length = 2
            elif first_upper_index is not None and first_upper_index > 0:
                min_length = max(min_length, 2)

            if first_upper_index is not None and first_upper_index > 0:
                for length in range(
                    min(5, len(tail_tokens) - first_upper_index),
                    0,
                    -1,
                ):
                    phrase = clean_candidate(
                        " ".join(
                            tail_tokens[first_upper_index : first_upper_index + length]
                        )
                    )
                    if phrase:
                        candidates.append(phrase)

            for length in range(min(5, len(tail_tokens)), min_length - 1, -1):
                phrase = clean_candidate(" ".join(tail_tokens[:length]))
                if phrase:
                    candidates.append(phrase)
            break

    if not article_led and allow_backoff:
        for length in range(min(5, len(tokens)), 0, -1):
            phrase = clean_candidate(" ".join(tokens[:length]))
            if phrase:
                candidates.append(phrase)

    seen = set()
    ordered = []
    for candidate in candidates:
        lowered = candidate.lower()
        if lowered in seen:
            continue
        seen.add(lowered)
        ordered.append(candidate)
    return ordered


def choose_headword(candidate: str, variants: list[str]) -> str:
    if candidate in variants:
        return candidate

    wants_upper = candidate[:1].isupper()
    if wants_upper:
        for variant in variants:
            if variant[:1].isupper():
                return variant
    else:
        for variant in variants:
            if variant == variant.lower():
                return variant

    return variants[0]


def resolve_headword(
    line: str,
    headword_map: dict[str, list[str]],
    *,
    allow_backoff: bool = True,
) -> str | None:
    for candidate in candidate_prefixes(line, allow_backoff=allow_backoff):
        variants = headword_map.get(candidate.lower())
        if variants:
            return choose_headword(candidate, variants)
    return None


def preprocess_line_for_source(spec: SourceSpec, raw_line: str) -> tuple[str | None, bool]:
    if not spec.requires_non_latin:
        return raw_line, True

    if not ARABIC_RE.search(raw_line):
        return None, True

    prefix = ARABIC_RE.split(raw_line, maxsplit=1)[0]
    prefix = prefix.split("Bsp.:", 1)[0]
    prefix = prefix.split("Def.:", 1)[0]
    prefix = re.sub(r"\s+", " ", prefix).strip()
    if not prefix:
        return None, True

    tokens = prefix.split()
    article_led = any(prefix.startswith(f"{article} ") for article in ARTICLE_PREFIXES)
    starts_lower = tokens and tokens[0][:1].islower()
    has_conjunction = any(token.lower() in {"und", "oder"} for token in tokens)
    title_like = tokens and all(
        token.lower() in {"und", "oder"} or token[:1].isupper() for token in tokens
    )
    allow_backoff = True
    if len(tokens) > 1 and not article_led:
        if starts_lower or has_conjunction or title_like:
            allow_backoff = False

    return prefix, allow_backoff


def build_rows(specs: list[SourceSpec], headword_map: dict[str, list[str]]) -> tuple[list[dict], dict]:
    rows = []
    stats = {"matched": 0, "duplicates": 0, "skipped": 0, "missing_source": 0}
    seen_headwords = set()
    level_orders: dict[str, int] = {}

    for spec in specs:
        if not spec.path.exists():
            stats["missing_source"] += 1
            continue

        reader = PdfReader(str(spec.path))
        current_lesson = None
        lesson_entry_index = 0

        for page in reader.pages:
            for raw_line in (page.extract_text() or "").splitlines():
                if is_noise(raw_line):
                    continue

                lesson_number = extract_lesson_number(raw_line)
                if lesson_number is not None:
                    current_lesson = lesson_number
                    lesson_entry_index = 0
                    continue

                if current_lesson is None:
                    continue

                normalized_line, allow_backoff = preprocess_line_for_source(spec, raw_line)
                if not normalized_line:
                    stats["skipped"] += 1
                    continue

                headword = resolve_headword(
                    normalized_line,
                    headword_map,
                    allow_backoff=allow_backoff,
                )
                if not headword:
                    stats["skipped"] += 1
                    continue

                if headword in seen_headwords:
                    stats["duplicates"] += 1
                    continue

                seen_headwords.add(headword)
                level_orders.setdefault(spec.cefr_level, 0)
                level_orders[spec.cefr_level] += 1
                lesson_entry_index += 1

                rows.append(
                    {
                        "headword": headword,
                        "cefr_level": spec.cefr_level,
                        "cefr_rank": spec.cefr_rank,
                        "learning_order": level_orders[spec.cefr_level],
                        "source": f"{spec.book_code}_l{current_lesson}",
                        "lesson_number": current_lesson,
                        "lesson_entry_index": lesson_entry_index,
                    }
                )
                stats["matched"] += 1

    return rows, stats


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, help="Path to write normalized JSON")
    parser.add_argument(
        "--database-url",
        default=os.environ.get("DATABASE_URL"),
        help="Database URL; defaults to DATABASE_URL env var",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.database_url:
        raise SystemExit("DATABASE_URL is required")

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with psycopg.connect(args.database_url) as conn:
        headword_map = build_headword_map(conn)

    rows, stats = build_rows(DEFAULT_SPECS, headword_map)
    with output_path.open("w", encoding="utf-8") as fh:
        json.dump(rows, fh, ensure_ascii=False, indent=2)

    print(
        json.dumps(
            {
                "output": str(output_path),
                "rows": len(rows),
                "matched": stats["matched"],
                "duplicates": stats["duplicates"],
                "skipped": stats["skipped"],
                "missing_source": stats["missing_source"],
                "levels": {
                    level: len([row for row in rows if row["cefr_level"] == level])
                    for level in sorted({row["cefr_level"] for row in rows})
                },
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
