import json
def parse(items):
    return tuple(map(json.loads, items))
