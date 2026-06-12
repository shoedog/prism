import functools

@functools.cache
def handler(x):
    return x

def run():
    handler(1)
