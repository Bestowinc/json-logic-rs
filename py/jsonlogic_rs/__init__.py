"""Python JSONLogic with a Rust Backend."""

__all__ = (
    "jsonlogic",
    "jsonlogic_serialized",
)

import json as _json

from .jsonlogic import apply as _apply


def apply(value, data=None, serializer=None, deserializer=None):
    """Run JSONLogic on a value and some data."""
    serializer = serializer if serializer is not None else _json.dumps
    deserializer = deserializer if deserializer is not None else _json.loads
    res = _apply(serializer(value), serializer(data))
    return deserializer(res)


def apply_serialized(value: str, data: str = None, deserializer=None):
    """Run JSONLogic on some already serialized value and optional data."""
    res = _apply(value, data if data is not None else "null")
    return deserializer(res)
