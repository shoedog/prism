"""Feasibility checks for the prototype's supported schema conjunctions."""

from math import ceil, floor


_TYPES = frozenset({"array", "boolean", "integer", "null", "number", "object", "string"})


def types(schemas: list[dict]) -> frozenset[str]:
    possible = _TYPES
    for schema in schemas:
        declared = schema.get("type")
        if declared is None:
            continue
        allowed = {declared} if isinstance(declared, str) else set(declared)
        if "number" in allowed:
            allowed.add("integer")
        if schema.get("nullable") is True:
            allowed.add("null")
        possible = possible.intersection(allowed)
    return possible


def type_name(possible: frozenset[str]) -> str | None:
    if possible == {"integer", "number"}:
        return "number"
    if len(possible) == 1:
        return next(iter(possible))
    return None


def limits(schemas: list[dict], minimum: str, maximum: str) -> tuple[int, int | float]:
    return (max((schema.get(minimum, 0) for schema in schemas), default=0),
            min((schema.get(maximum, float("inf")) for schema in schemas), default=float("inf")))


def _scalar_type(value: object) -> str:
    return {
        type(None): "null", bool: "boolean", int: "integer", float: "number",
        str: "string", list: "array", dict: "object",
    }[type(value)]


def _bounds(schemas: list[dict]) -> tuple[tuple[float, bool], tuple[float, bool]]:
    lower = (-float("inf"), False)
    upper = (float("inf"), False)
    for schema in schemas:
        for key, inclusive, exclusive in (
            ("minimum", schema.get("minimum"), schema.get("exclusiveMinimum")),
            ("maximum", schema.get("maximum"), schema.get("exclusiveMaximum")),
        ):
            candidates = []
            if inclusive is not None:
                candidates.append((inclusive, exclusive is True))
            if type(exclusive) in (int, float):
                candidates.append((exclusive, True))
            if key == "minimum":
                lower = max(lower, *candidates) if candidates else lower
            else:
                upper = min([upper, *candidates], key=lambda bound: (bound[0], not bound[1]))
    return lower, upper


def _finite_domain(schemas: list[dict]) -> list | None:
    domains = [schema["enum"] for schema in schemas if "enum" in schema]
    domains.extend([schema["const"]] for schema in schemas if "const" in schema)
    if not domains:
        return None

    def same_value(left, right):
        numeric = type(left) in (int, float) and type(right) in (int, float)
        return (numeric or type(left) is type(right)) and left == right

    return [value for value in domains[0]
            if all(any(same_value(value, item) for item in domain) for domain in domains[1:])]


def feasible_types(schemas: list[dict]) -> frozenset[str]:
    possible = types(schemas)
    (low, lower_open), (high, upper_open) = _bounds(schemas)
    if low > high or (low == high and (lower_open or upper_open)):
        possible -= {"number", "integer"}
    else:
        least = low if low == -float("inf") else floor(low) + 1 if lower_open else ceil(low)
        greatest = high if high == float("inf") else ceil(high) - 1 if upper_open else floor(high)
        if least > greatest:
            possible -= {"integer"}
    min_items, max_items = limits(schemas, "minItems", "maxItems")
    if min_items > max_items:
        possible -= {"array"}
    min_length, max_length = limits(schemas, "minLength", "maxLength")
    if min_length > max_length:
        possible -= {"string"}
    domain = _finite_domain(schemas)
    if domain is None:
        return possible

    present = set()
    for value in domain:
        kind = _scalar_type(value)
        if kind in {"number", "integer"}:
            if not ((value > low or value == low and not lower_open)
                    and (value < high or value == high and not upper_open)):
                continue
            if kind == "number" and value.is_integer():
                kind = "integer"
        if kind == "string" and not min_length <= len(value) <= max_length:
            continue
        present.add(kind)
    return possible.intersection(present)


def effective_enum(schemas: list[dict]) -> list | None:
    if not any("enum" in schema for schema in schemas):
        return None
    domain = _finite_domain(schemas)
    return [value for value in domain if feasible_types([*schemas, {"const": value}])]
