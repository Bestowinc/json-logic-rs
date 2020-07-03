"""Python JSONLogic with a Rust Backend."""

__all__ = (
    "apply",
    "apply_serialized",
)

import json as _json
import sys as _sys

try:
    from .jsonlogic import apply as _apply
except ImportError:
    # See https://docs.python.org/3/library/os.html#os.add_dll_directory
    # for why this is here.
    if _sys.platform.startswith("win"):
        import os
        from pathlib import Path
        if hasattr(os, "add_dll_directory"):
            os.add_dll_directory(str(Path(__file__).parent))
        from .jsonlogic import apply as _apply
    else:
        raise


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
