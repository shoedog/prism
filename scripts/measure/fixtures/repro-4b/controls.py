"""Make the five predeclared trace controls from the retained fixture capture."""

from collections import defaultdict
import argparse
import gzip
import json
from pathlib import Path


def edge_key(edge):
    return tuple((edge[end]["file"], edge[end]["line"], edge[end]["start_byte"])
                 for end in ("from", "to"))


def place(endpoint):
    path = endpoint["path"]
    return ".".join([path["base"], *path["fields"]]) + f"@{endpoint['line']}:{endpoint['start_byte']}"


def control_key(fields):
    return tuple((fields[2], *map(int, value.rsplit("@", 1)[1].split(":"))) for value in fields[4:])


def controls(base, head, stderr):
    records = stderr.decode("utf-8").splitlines(keepends=True)
    before, after = defaultdict(list), defaultdict(list)
    for index, dump in ((before, base), (after, head)):
        for wire in dump:
            index[edge_key(wire)].append(wire)
    alias, d3 = [], []
    for i, line in enumerate(records):
        fields = line.rstrip("\n").split("\t")
        if fields[:2] == ["rd-census", "kill:alias"]:
            alias.append((i, fields))
        elif fields[:2] == ["rd-census", "alias-d3"]:
            d3.append((i, fields))
    for candidate_index, candidate in alias:
        target = control_key(candidate)[1]
        alternatives = [key for key, rows in after.items()
                        if len(rows) == 1 and rows == before.get(key)
                        and key[0][0] == key[1][0] and key[1][:2] == target[:2]
                        and key[1][2] != target[2]]
        if alternatives:
            kill_index, kill = candidate_index, candidate
            neighbor = after[min(alternatives)][0]
            break
    else:
        raise ValueError("fixture has no unchanged same-line neighbor of a kill:alias trace")

    d3_index, other = d3[0]

    def emit(fields):
        return "\t".join(fields) + "\n"

    def change(edits):
        return "".join(edits.get(i, row) for i, row in enumerate(records)).encode()

    moved = emit([*kill[:2], neighbor["to"]["file"], kill[3], place(neighbor["from"]), place(neighbor["to"])])
    path, coordinate = kill[5].rsplit("@", 1)
    line, start = coordinate.split(":")
    shifted = emit([*kill[:5], f"{path}@{line}:{int(start) + 1}"])
    return {
        "c1": change({kill_index: moved}),
        "c2": change({kill_index: ""}),
        "c3": change({kill_index: emit([kill[0], other[1], *kill[2:]]),
                      d3_index: emit([other[0], kill[1], *other[2:]])}),
        "c4": stderr + moved.encode(),
        "c5": change({kill_index: shifted}),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    capture = Path(__file__).parent / "capture"
    base, head = ([json.loads(line) for line in gzip.decompress(
        (capture / f"{arm}.edges.jsonl.gz").read_bytes()).splitlines()]
        for arm in ("base", "head"))
    trace = gzip.decompress((capture / "head.edges.stderr.gz").read_bytes())
    edits = controls(base, head, trace)
    args.out.mkdir()
    for name, content in edits.items():
        (args.out / f"{name}.stderr").write_bytes(content)


if __name__ == "__main__":
    main()
