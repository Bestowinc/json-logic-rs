"""Test the python distribution."""

import json
import typing as t
from pathlib import Path

import jsonlogic_rs


TEST_FILE = Path(__file__).parent / "data/tests.json"

JsonValue = t.Union[dict, list, str, bool, None]


class TestCase(t.NamedTuple):
    """A test case from the JSON."""

    logic: JsonValue
    data: JsonValue
    exp: JsonValue


def load_tests() -> t.List[TestCase]:
    """Load the test json into a series of cases."""
    with open(TEST_FILE) as f:
        raw_cases = filter(lambda case: not isinstance(case, str), json.load(f))
        return list(map(lambda case: TestCase(*case), raw_cases))


def run_tests() -> None:
    """Run through the tests and assert we get the right output."""
    for idx, case in enumerate(load_tests()):
        result = jsonlogic_rs.apply(case.logic, case.data)
        assert result == case.exp, f"Failed test case {idx}: {case}"


if __name__ == "__main__":
    run_tests()
