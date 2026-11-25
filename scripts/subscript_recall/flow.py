"""Language-independent syntax events. Paths contain a lexical binding and selectors."""

from collections import Counter, defaultdict, deque
from dataclasses import dataclass, field

INDEX = "[]"
READ_LIMIT = 8
OWNER_LIMIT = 256
RAW_LIMIT = 1_000_000
MAX_COUNTER = (1 << 64) - 1

REASONS = (
    "completion_unproven", "reentry_required", "eager_effect_in_target",
    "eager_effect_at_read", "replacement", "deletion",
)
BITS = {name: 1 << index for index, name in enumerate(REASONS)}


def prefix(left, right):
    return len(left) <= len(right) and right[:len(left)] == left


@dataclass
class Endpoint:
    span: tuple
    line: int
    path: tuple
    binding: object
    role: str
    flags: set = field(default_factory=set)
    event: int = -1
    loops: tuple = ()
    start_capture: int = -1
    container_kind: str = ""
    operand_span: tuple = ()
    statement_line: int = 0

    @property
    def storage(self):
        return (self.binding, *self.static_path[1:])

    @property
    def static_path(self):
        return self.path[:self.path.index(INDEX)] if INDEX in self.path else self.path

    @property
    def binding_path(self):
        return ((self.binding, self.path[0]), *self.path[1:len(self.static_path)])

    def location_key(self, access):
        path = self.path
        if access == "def" and INDEX in path:
            path = path[:path.index(INDEX) + 1]
        return access, self.line, path

    def row(self):
        row = dict(start=self.span[0], end=self.span[1], line=self.line,
                   statement_line=self.statement_line or self.line,
                   path=self.path, binding=self.binding, role=self.role, flags=sorted(self.flags))
        if self.operand_span:
            row["operand_span"] = self.operand_span
        if self.container_kind:
            row["container_kind"] = self.container_kind
        return row


@dataclass
class Event:
    kind: str
    successors: tuple
    span: tuple = ()
    data: object = None


class Graph:
    def __init__(self):
        self.events = []
        self.writes = {}
        self.reads = {}
        self.locations = defaultdict(set)
        self.unordered_locations = defaultdict(set)
        self.flags = Counter()

    def event(self, kind, successors=(), span=(), data=None):
        index = len(self.events)
        self.events.append(Event(kind, tuple(successors), span, data))
        return index

    def branch(self, truth, yes, no, span=()):
        choices = ((yes, "true"), (no, "false"))
        if truth is not None:
            choices = (choices[0 if truth else 1],)
        return self.event("branch", (dst for dst, _ in choices), span,
                          dict(reversed(choices)))

    def endpoint(self, endpoint):
        inventory = self.writes if endpoint.role in ("write", "aug_write") else self.reads
        previous = inventory.setdefault(endpoint.span, endpoint)
        previous.flags.update(endpoint.flags)
        return previous

    def location(self, access, span, line, path):
        self.locations[(access, line, path)].add(span)

    def collisions(self):
        for access, points in (("def", self.writes), ("use", self.reads)):
            for endpoint in points.values():
                places = self.locations[endpoint.location_key(access)] | {endpoint.span}
                if len(places) > 1:
                    endpoint.flags.add("same_line_collision")

    def reachable(self, entry):
        # Track whether an endpoint is only reachable through a phase we could not certify.
        reach = self.walk([(entry, 0, None)], self.transition)
        return {index: {mask for mask, _ in states} for index, states in reach.items()}

    def transition(self, event, mask, capture):
        if event.kind in ("unproven", "unordered"):
            mask |= BITS["completion_unproven"]
        return mask, capture

    def walk(self, starts, transfer, stop=None, trail=None):
        pending = deque(starts)
        visiting = defaultdict(set)
        while pending:
            index, mask, capture = pending.popleft()
            states = visiting[index]
            state = (mask, capture)
            if any(progress == capture and existing & mask == existing
                   for existing, progress in states):
                continue
            states.difference_update({
                (existing, progress) for existing, progress in states
                if progress == capture and existing & mask == mask
            })
            states.add(state)
            if index == stop:
                continue
            event = self.events[index]
            next_mask, next_capture = transfer(event, mask, capture)
            for following in event.successors:
                item = (following, next_mask, next_capture)
                if trail is not None:
                    trail.setdefault(item, (index, mask, capture))
                pending.append(item)
        return visiting

    def capture_masks(self, write, reach):
        if write.start_capture not in reach:
            return set()

        def transfer(event, mask, captured):
            mask, captured = self.transition(event, mask, captured)
            if event.kind == "capture" and event.data[0] == write.span:
                captured = event.data[1]
            if captured and event.kind in ("replace", "delete", "clear", "index_replace"):
                effect = event.data
                if event.kind == "clear":
                    detached = prefix((*effect, INDEX), captured)
                else:
                    detached = prefix(effect, captured)
                if detached:
                    mask |= BITS["eager_effect_in_target"]
            if captured and event.kind in ("unproven", "unordered"):
                mask |= BITS["eager_effect_in_target"]
            return mask, captured

        states = self.walk(
            # The route starts at W: how W itself was reached (an earlier unsupported compound) is not part of it.
            [(write.start_capture, 0, ())],
            transfer, stop=write.event,
        )
        return {mask for mask, _ in states.get(write.event, ())}

    def routes(self, write, reach, detailed):
        masks = self.capture_masks(write, reach)

        def transfer(event, mask, capture):
            mask, capture = self.transition(event, mask, capture)
            if event.kind == "loop_entry" and event.data in write.loops:
                mask |= BITS["reentry_required"]
            if event.kind in ("replace", "delete", "clear"):
                effect = event.data
                if event.kind != "replace" and INDEX in effect:
                    effect = effect[:effect.index(INDEX)]
                if prefix(effect, write.storage):
                    mask |= BITS["replacement" if event.kind == "replace" else "deletion"]
            if event.kind == "unordered" and any(prefix(p, write.storage) for p in event.data):
                mask |= BITS["eager_effect_at_read"]
            return mask, capture

        starts = [(index, mask, None) for index in self.events[write.event].successors
                  for mask in sorted(masks)]
        trail = dict.fromkeys(starts) if detailed else None
        return self.walk(starts, transfer, trail=trail), trail

    def phase_trace(self, trail, write, read, mask):
        point = (read.event, mask, None)
        following = None
        trace = []
        while point is not None:
            event = self.events[point[0]]
            if event.span:
                phase = {"phase": event.kind, "span": event.span}
                if event.kind == "branch" and following in event.data:
                    phase["arm"] = event.data[following]
                if event.kind in ("break", "continue"):
                    phase["target"] = event.data
                trace.append(phase)
            following = point[0]
            point = trail.get(point)
        trace.append({"phase": "commit", "span": write.span})
        return trace[::-1]


def matching(storage, buckets):
    for size in range(1, len(storage) + 1):
        yield from buckets.get(storage[:size], ())


def work(storages, buckets):
    counts = Counter()
    for path in storages:
        for size in range(1, len(path) + 1):
            bucket = path[:size]
            if bucket in buckets:
                counts[bucket] += 1
    return (sum(len(buckets[b]) * n for b, n in counts.items()),
            sum(len(buckets[b]) for b in counts))


def pair_rows(graph, writes, read_buckets, reach, detailed):
    for write in writes:
        routes, trail = None, None
        for read in matching(write.binding_path, read_buckets):
            if write.line == read.line or (write.statement_line and
                                           write.statement_line == read.statement_line):
                yield write, read, "same_statement_line", (), None
                continue
            if routes is None:
                routes, trail = graph.routes(write, reach, detailed)
            masks = {mask for mask, _ in routes.get(read.event, ())}
            if 0 in masks and write.span[0] < read.span[0]:
                yield write, read, "surviving_route", (), (trail, 0)
                continue
            reentry = BITS["reentry_required"]
            # A backward byte route is useful evidence only when it needs another entry.
            if write.span[0] >= read.span[0]:
                masks = {mask for mask in masks if mask & reentry}
            flags = {name for name, bit in BITS.items() if any(mask & bit for mask in masks)}
            if reentry not in masks:
                flags.discard("reentry_required")
            if not any(prefix(p, write.storage)
                       for p in graph.unordered_locations.get(read.statement_line, ())):
                flags.discard("eager_effect_at_read")
            reason = next((name for name in REASONS if name in flags), "no_ordered_route")
            if "eager_effect_in_target" in flags:
                reason = "eager_effect_in_target"
            proof = (trail, reentry) if reentry in masks else None
            yield write, read, reason, sorted(flags), proof


def measure(graph, entry, detailed, recorded=()):
    graph.collisions()
    writes = sorted(graph.writes.values(), key=lambda endpoint: endpoint.span)
    reads = sorted(graph.reads.values(), key=lambda endpoint: endpoint.span)
    eligible_writes = [w for w in writes if not w.flags]
    read_buckets, spelling_buckets = defaultdict(list), defaultdict(list)
    for read in reads:
        spelling_buckets[read.static_path].append(read)
        if not read.flags:
            read_buckets[read.binding_path].append(read)
    spelled, spelled_reads = work((w.static_path for w in writes), spelling_buckets)
    raw, raw_reads = work((w.binding_path for w in eligible_writes), read_buckets)
    counts = dict(writes=len(writes), reads=len(reads), eligible_writes=len(eligible_writes),
                  eligible_reads=sum(map(len, read_buckets.values())), raw_pairs=min(raw, MAX_COUNTER),
                  raw_count_saturated=int(raw > MAX_COUNTER),
                  raw_pair_cap=int(raw > RAW_LIMIT), reads_over_fanin=0, owner_edge_cap=0,
                  endpoint_refusals_unenumerated=spelled - raw if spelled > RAW_LIMIT else 0,
                  raw_pairs_capped=min(raw, MAX_COUNTER) if raw > RAW_LIMIT else 0,
                  surviving_pairs=0, surviving_reads=0, pairs_after_fanin=0, reads_after_fanin=0,
                  would_add=0, would_add_at_c3=0)
    endpoint_flags = Counter(flag for endpoint in writes + reads for flag in endpoint.flags)
    flags = endpoint_flags + graph.flags
    refusals, endpoint_pair_refusals = Counter(), Counter()
    pair_flags = defaultdict(Counter)
    read_costs, recorded_costs = defaultdict(set), defaultdict(set)
    recorded = set(recorded)
    survivors = defaultdict(list)
    sources_per_read = Counter()
    pairs = []
    if spelled <= RAW_LIMIT:
        for write in writes:
            for read in matching(write.static_path, spelling_buckets):
                reasons = write.flags | read.flags
                if write.binding is not None and read.binding is not None and write.binding != read.binding:
                    reasons = reasons | {"different_binding"}
                if not reasons:
                    continue
                endpoint_pair_refusals.update(reasons)
                read_costs[read.span].update(reasons)
                id_ = (write.span[0], read.span)
                if id_ in recorded:
                    recorded_costs[id_].update(reasons)
                if detailed:
                    pairs.append(dict(w=write.span, r=read.span, role=read.role,
                                      verdict="refused", reason="endpoint_or_binding", flags=sorted(reasons)))
    if 0 < raw <= RAW_LIMIT:
        reach = graph.reachable(entry)
        for write, read, reason, reasons, proof in pair_rows(
            graph, eligible_writes, read_buckets, reach, detailed,
        ):
            row = None
            if detailed or (reason == "surviving_route" and sources_per_read[read.span] < READ_LIMIT):
                row = dict(w=write.span, r=read.span, role=read.role, verdict="refused",
                           reason=reason, flags=reasons)
                if detailed and proof:
                    trail, mask = proof
                    row["route"] = graph.phase_trace(trail, write, read, mask)
            if reason == "surviving_route":
                sources_per_read[read.span] += 1
                if row:
                    survivors[read.span].append(row)
            else:
                refusals[reason] += 1
                costs = reasons or (reason,)
                flags.update(costs)
                pair_flags[read.role].update(costs)
                read_costs[read.span].update(costs)
                id_ = (write.span[0], read.span)
                if id_ in recorded:
                    recorded_costs[id_].update(costs)
            if detailed:
                pairs.append(row)
    counts["surviving_reads"] = len(sources_per_read)
    counts["surviving_pairs"] = sum(sources_per_read.values())
    remaining = []
    for read_span, rows in survivors.items():
        if sources_per_read[read_span] > READ_LIMIT:
            counts["reads_over_fanin"] += 1
            read_costs[read_span].add("fanin_cap")
            for row in rows:
                row["reason"] = "fanin_cap"
            refusals["fanin_cap"] += sources_per_read[read_span]
        else:
            counts["reads_after_fanin"] += 1
            remaining.extend(rows)
    counts["pairs_after_fanin"] = len(remaining)
    if len(remaining) > OWNER_LIMIT:
        counts["owner_edge_cap"] = 1
        refusals["owner_edge_cap"] += len(remaining)
        for row in remaining:
            row["reason"] = "owner_edge_cap"
            read_costs[row["r"]].add("owner_edge_cap")
        remaining.clear()
    for row in remaining:
        row["verdict"] = "would_add"
    counts["would_add"] = len(remaining)
    attrition = {
        "spelled_storage": [spelled, spelled_reads],
        "syntax_endpoints_and_binding": [raw, raw_reads],
        "raw_pair_budget": [raw, raw_reads] if raw <= RAW_LIMIT else [0, 0],
        "route": [counts["surviving_pairs"], counts["surviving_reads"]],
        "fanin_budget": [counts["pairs_after_fanin"], counts["reads_after_fanin"]],
        "owner_edge_budget": [len(remaining), len({r["r"] for r in remaining})],
    }
    result = dict(counts=counts, flags=dict(flags), endpoint_flags=dict(endpoint_flags),
                  endpoint_pair_refusals=dict(endpoint_pair_refusals), attrition=attrition,
                  pair_flags=dict(pair_flags), pair_refusals=dict(refusals),
                  would_add_by_role=dict(Counter(row["role"] for row in remaining)))
    if detailed:
        result.update(writes=[w.row() for w in writes], reads=[r.row() for r in reads],
                      pairs=sorted(pairs, key=lambda row: (row["w"], row["r"])),
                      would_add=[{k: row[k] for k in ("w", "r", "role")}
                                 for row in sorted(remaining, key=lambda x: (x["w"], x["r"]))])
    return result, remaining, read_costs, recorded_costs
