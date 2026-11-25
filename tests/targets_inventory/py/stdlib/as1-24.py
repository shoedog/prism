try:
    import tomllib
except ImportError:
    import tomli as tomllib
def parse(stream):
    return tomllib.load(stream)
