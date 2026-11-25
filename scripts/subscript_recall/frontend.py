"""Shared lowering operations for syntax frontends."""

from flow import Endpoint, Graph, INDEX

EXCLUDED_CALLS = {"len", "cap", "id", "Object.is", "delete", "clear"}


def literal_boolean(values, conjunction):
    decisive = not conjunction
    unknown = False
    for value in values:
        if value is decisive:
            return decisive
        unknown |= value is None
    return None if unknown else conjunction


def merge_owner_locations(file, owner, graph, line_bounds):
    """Include syntax occurrences in other callable owners on endpoint lines."""
    keys_by_line = {}
    for access, endpoints in (("def", graph.writes), ("use", graph.reads)):
        for endpoint in endpoints.values():
            keys_by_line.setdefault(endpoint.line, set()).add(endpoint.location_key(access))
    if not keys_by_line:
        return
    file.locations[owner] = {key: frozenset(spans) for key, spans in graph.locations.items()}
    for neighbors in file.owners.values():
        for other in neighbors:
            if other == owner or getattr(other, "has_error", False):
                continue
            first_line, last_line = line_bounds(other)
            keys = {key for line in keys_by_line if first_line <= line <= last_line
                    for key in keys_by_line[line]}
            if not keys:
                continue
            if other not in file.locations:
                file.locations[other] = file.build_owner(other)[0].locations
            for key in keys:
                graph.locations[key].update(file.locations[other].get(key, ()))


class Frontend:
    def __init__(self, source):
        self.source = source
        self.graph = Graph()
        self.emit = self.graph.event
        self.branch = self.graph.branch

    def text(self, node):
        start, end = self.span(node)
        return self.source[start:end].decode()

    def lower_list(self, nodes, end, lower):
        for node in reversed(list(nodes)):
            end = lower(node, end)
        return end

    def uncertain(self, node, *exits):
        return self.emit("unproven", exits, self.span(node))

    def path(self, node, *, indexed=False):
        parts = self.parts(node)
        if parts is None:
            return None
        root, links = parts
        path = (self.text(root), *(name for _, name, _ in links))
        if not indexed and INDEX in path:
            return None
        return root, links, path

    def location(self, node, access):
        parsed = self.path(node, indexed=access == "def")
        if parsed:
            _, _, path = parsed
            if INDEX in path:
                path = path[:path.index(INDEX) + 1]
            self.graph.location(access, self.span(node), self.line(node), path)

    def endpoint(self, node, role, loops=(), flags=(), operand_span=()):
        parsed = self.path(node, indexed=True)
        if not parsed:
            return None
        root, links, path = parsed
        binding, root_flag = self.binding(root)
        flags = set(flags)
        if root_flag:
            flags.add(root_flag)
        if INDEX in path and any(name != INDEX for name in path[path.index(INDEX) + 1:]):
            flags.add("field_after_index")
        if self.excluded_chain(node):
            flags.add("unsupported_chain")
        point = Endpoint(self.span(node), self.line(node), path, binding, role, flags, loops=loops,
                         operand_span=operand_span, statement_line=self.statement_line(node))
        self.annotate_container(point, root, links)
        return self.graph.endpoint(point)

    def annotate_container(self, point, root, links):
        pass

    def statement_line(self, node):
        return self.line(node)

    def excluded_chain(self, node):
        return False

    def read(self, node, role, end, flags=()):
        parsed = self.path(node)
        if role and parsed and "deferred_operand" not in flags:
            point = self.endpoint(node, role, flags=flags)
            self.location(node, "use")
            point.event = self.emit("read", (end,), point.span)
            end = point.event
        return end

    def mutation(self, node, end, kind="replace"):
        parsed = self.path(node, indexed=True)
        if not parsed:
            return end
        root, _, path = parsed
        binding, flag = self.binding(root)
        if flag:
            return end
        storage = (binding, *path[1:])
        if kind == "replace" and INDEX in storage:
            kind = "index_replace"
        return self.emit(kind, (end,), self.span(node), storage)

    def write(self, node, lower_expression, rhs, end, loops, *, compound=False, rhs_first=False):
        parsed = self.path(node, indexed=True)
        if not parsed or INDEX not in parsed[2]:
            return None
        root, links, _ = parsed
        role = "aug_write" if compound else "write"
        point = self.endpoint(node, role, loops)
        self.location(node, "def")
        if point.flags:
            return None
        # Also expose the commit as a key-blind change to any capture already in progress.
        end = self.mutation(node, end)
        point.event = self.emit("commit", (end,), point.span)
        cursor = point.event
        if not rhs_first:
            cursor = rhs(cursor)
        chain = (point.binding, *point.path[1:])
        for offset, (part, name, index) in reversed(list(enumerate(links, 1))):
            if offset < len(links):
                cursor = self.emit("capture", (cursor,), self.span(part),
                                   (point.span, chain[:offset + 1]))
            if name == INDEX:
                cursor = lower_expression(index, cursor)
        point.start_capture = self.emit("capture", (cursor,), self.span(root),
                                        (point.span, chain[:1]))
        return rhs(point.start_capture) if rhs_first else point.start_capture
