import json


def parse(items):
    return [json.loads(t) for t in items]
