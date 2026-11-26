import json


def parse(t):
    f = lambda s: json.loads(s)
    return f(t)
