def src():
    return "TAINT-7f3a"


def sink(value):
    pending = [value]
    seen = {}
    while pending:
        value = pending.pop()
        if isinstance(value, str):
            if str.__contains__(value, src()):
                print("OBSERVE reaches")
                return
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
    print("OBSERVE not")
