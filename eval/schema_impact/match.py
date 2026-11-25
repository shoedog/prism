"""Match canonical client URL patterns against the operation index."""

from .consumer_ts import ClientCall
from .schema import SchemaIndex


def _segments(path: str) -> list[str]:
    return path[1:].split("/") if path != "/" else []


def bind(call: ClientCall, idx: SchemaIndex) -> tuple[str, list[str], str | None]:
    if call.reason is not None:
        return "unbound", [], call.reason
    if call.method is None or call.pattern is None:
        return "unbound", [], "no_operation"

    client = _segments(call.pattern)
    prefix = f"op:{call.method} "
    top = []
    top_score = -1
    for operation in idx.operations:
        if not operation.startswith(prefix):
            continue
        schema = _segments(operation[len(prefix):])
        if len(client) != len(schema):
            continue
        pairs = list(zip(client, schema))
        if any(left != right and "{}" not in (left, right) for left, right in pairs):
            continue
        score = sum(left == right for left, right in pairs)
        if score > top_score:
            top_score = score
            top.clear()
        if score == top_score:
            parameter_on_literal = any(left == "{}" and right != "{}" for left, right in pairs)
            top.append((operation, parameter_on_literal))

    if not top:
        return "unbound", [], "no_operation"
    ids = sorted(operation for operation, _ in top)
    if len(top) > 1:
        return "candidate", ids, "ambiguous_operation"
    if top[0][1]:
        return "candidate", ids, "client_parameter_on_literal"
    return "bound", ids, None
