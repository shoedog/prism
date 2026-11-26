import json
def f():
    def inner(x: int | json.loads("null")):
        return x
    return inner
