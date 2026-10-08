"""Verify local SQL seeds and optional exported live scale JSON without a database."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COLUMNS = ["scale_id", "scale_name", "scale_description", "min_score", "max_score", "severity_ranges", "questions"]


def read_sql_scales(path):
    text = Path(path).read_text(encoding="utf-8-sig")
    start = text.index("INSERT INTO depression_scales")
    start = text.index("VALUES", start) + len("VALUES")
    end = text.find("ON DUPLICATE KEY UPDATE", start)
    if end < 0:
        end = text.index(";", start)
    # This limited parser accepts only literal seed rows and JSON constructors.
    tokens = re.findall(r"'(?:''|\\.|[^'])*'|[A-Za-z_]+|-?\d+|[(),]", text[start:end])
    pos = 0

    def value():
        nonlocal pos
        token = tokens[pos]
        pos += 1
        if token.startswith("'"):
            return token[1:-1].replace("''", "'")
        if re.fullmatch(r"-?\d+", token):
            return int(token)
        if token.lower() in ("true", "false", "null"):
            return {"true": True, "false": False, "null": None}[token.lower()]
        if token == "(":
            items = arguments()
            return items
        if token in ("JSON_ARRAY", "JSON_OBJECT"):
            assert tokens[pos] == "("
            pos += 1
            items = arguments()
            if token == "JSON_ARRAY":
                return items
            assert len(items) % 2 == 0
            return dict(zip(items[::2], items[1::2]))
        raise ValueError(f"unexpected token {token}")

    def arguments():
        nonlocal pos
        items = []
        if tokens[pos] == ")":
            pos += 1
            return items
        while True:
            items.append(value())
            token = tokens[pos]
            pos += 1
            if token == ")":
                return items
            assert token == ","

    rows = []
    while pos < len(tokens):
        row = value()
        assert len(row) == len(COLUMNS), "scale column mismatch"
        rows.append(dict(zip(COLUMNS, row)))
        if pos < len(tokens):
            assert tokens[pos] == ","
            pos += 1
    return rows


def validate(rows):
    expected = {"PHQ-9": (9, 0, 27, set()), "SDS": (20, 20, 80, {2, 5, 6, 11, 12, 14, 16, 17, 18, 20}),
                "CES-D": (20, 0, 60, {4, 8, 12, 16})}
    assert len(rows) == 3 and {r["scale_name"] for r in rows} == set(expected)
    assert len({r["scale_id"] for r in rows}) == 3
    for row in rows:
        count, low, high, reversed_ids = expected[row["scale_name"]]
        questions = row["questions"]
        assert len(questions) == count, row["scale_name"]
        assert [q["id"] for q in questions] == list(range(1, count + 1))
        assert {q["id"] for q in questions if q.get("reverse_scored")} == reversed_ids
        option_scores = list(range(1, 5)) if row["scale_name"] == "SDS" else list(range(4))
        for q in questions:
            assert q["text"].strip()
            assert all(o["label"].strip() for o in q["options"])
            scores = [o["score"] for o in q["options"]]
            assert scores == (option_scores[::-1] if q["id"] in reversed_ids else option_scores)
        assert sum(min(o["score"] for o in q["options"]) for q in questions) == low == row["min_score"]
        assert sum(max(o["score"] for o in q["options"]) for q in questions) == high == row["max_score"]
        ranges = row["severity_ranges"]
        coverage = []
        for r in ranges:
            assert r["level"].strip()
            coverage.extend(range(r["min"], r["max"] + 1))
        assert sorted(coverage) == list(range(low, high + 1)), row["scale_name"]
        if row["scale_name"] == "SDS":
            for raw in range(low, high + 1):
                # Integer round-half-up of raw * 1.25, consistent with the seed's raw cutoffs.
                standard = (raw * 5 + 2) // 4
                expected_grade = 0 if standard < 53 else 1 if standard < 63 else 2 if standard < 73 else 3
                assert ranges[expected_grade]["min"] <= raw <= ranges[expected_grade]["max"]
        print(f"PASS {row['scale_name']}: {count} questions, option scores, reverse items, bounds and grades")


def validate_mock_assessments(scales):
    text = (ROOT / "database/sql/mock.sql").read_text(encoding="utf-8-sig")
    start = text.index("INSERT INTO depression_assessments")
    section = text[start:text.index(";", start)]
    rows = re.findall(r"\((\d+),\s*(\d+),\s*(\d+),\s*'([^']+)',\s*'([^']+)',\s*(\d+),", section)
    assert len(rows) == 5, "expected five mock assessments"
    by_id = {row["scale_id"]: row for row in scales}
    for assessment_id, _, scale_id, _, encoded, total in rows:
        questions = by_id[int(scale_id)]["questions"]
        answers = json.loads(encoded)
        assert set(answers) == {str(q["id"]) for q in questions}, assessment_id
        for q in questions:
            score = answers[str(q["id"])]
            assert type(score) is int and score in {o["score"] for o in q["options"]}, assessment_id
        assert sum(answers.values()) == int(total), assessment_id
    print("PASS five mock assessments: complete answers, legal scores and matching totals")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--live-json", type=Path)
    args = parser.parse_args()
    canonical = read_sql_scales(ROOT / "database/sql/seed_psychology_depression.sql")
    mock = read_sql_scales(ROOT / "database/sql/mock.sql")
    validate(canonical)
    assert mock == canonical, "mock scale definitions drifted from canonical seed"
    print("PASS mock.sql matches canonical scale definitions")
    validate_mock_assessments(mock)
    if args.live_json:
        live = json.loads(args.live_json.read_text(encoding="utf-8"))
        validate(live)
        assert live == canonical, "live scale definitions differ from canonical seed"
        print("PASS live database matches canonical scale definitions")


if __name__ == "__main__":
    main()
