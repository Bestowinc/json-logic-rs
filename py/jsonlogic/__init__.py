"""Python JSONLogic with a Rust Backend."""

__all__ = (
    "jsonlogic",
    "jsonlogic_serialized",
)

import json

from .jsonlogic import jsonlogic as _jsonlogic


def jsonlogic(value, data=None, serializer=None, deserializer=None):
    """Run JSONLogic on a value and some data."""
    serializer = serializer if serializer is not None else json.dumps
    deserializer = deserializer if deserializer is not None else json.loads
    res = _jsonlogic(serializer(value), serializer(data))
    return deserializer(res)


def jsonlogic_serialized(value, data=None, deserializer=None):
    """Run JSONLogic on some already serialized value and optional data."""
    res = _jsonlogic(value, data if data is not None else "null")
    return deserializer(res)
