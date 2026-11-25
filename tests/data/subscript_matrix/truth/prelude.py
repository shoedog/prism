def src():
    return "TAINT-7f3a"


_MIXED_TOKENS = ("TAINT-A-4b19", "TAINT-B-28cd")
_MIXED_CALLS = [0, 0]
_MIXED = False
_MIXED_SINKS = 0


def _mixed_source(index):
    _MIXED_CALLS[index] += 1
    return _MIXED_TOKENS[index]


def src_a():
    return _mixed_source(0)


def src_b():
    return _mixed_source(1)


def _check_mixed_sources():
    if _MIXED_CALLS != [1, 1]:
        raise ValueError(f"mixed sources must each execute once, got {_MIXED_CALLS}")
    if _MIXED_SINKS != 1:
        raise ValueError(f"expected one valid observation from sink, got {_MIXED_SINKS}")


def sink(value):
    global _MIXED_SINKS
    if _MIXED:
        _MIXED_SINKS += 1
    tokens = _MIXED_TOKENS if _MIXED else (src(),)
    hits = [False] * len(tokens)
    pending = [value]
    seen = {}
    while pending:
        value = pending.pop()
        if isinstance(value, str):
            for index, token in enumerate(tokens):
                hits[index] |= str.__contains__(value, token)
            if all(hits):
                break
        if id(value) in seen:
            continue
        # Retain visited objects while instance dictionaries may be materialized.
        seen[id(value)] = value
        if isinstance(value, dict):
            for key, item in dict.items(value):
                pending.extend((key, item))
        elif isinstance(value, list):
            pending.extend(list.__iter__(value))
        elif isinstance(value, tuple):
            pending.extend(tuple.__iter__(value))
        elif isinstance(value, set):
            pending.extend(set.__iter__(value))
        try:
            attributes = object.__getattribute__(value, "__dict__")
        except AttributeError:
            continue
        pending.append(attributes)
    if _MIXED:
        print(f"OBSERVE A {int(hits[0])} B {int(hits[1])}")
    else:
        print("OBSERVE reaches" if hits[0] else "OBSERVE not")
