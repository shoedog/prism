from os import environ as __annotations__
def inspect(key):
    class Outer:
        first = __annotations__.get(key)
        class Inner:
            second = __annotations__.get(key)
            field: str
