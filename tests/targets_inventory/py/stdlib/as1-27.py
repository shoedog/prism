from configparser import ConfigParser, RawConfigParser
def make() -> RawConfigParser:
    return ConfigParser()
def read(path):
    parser = make()
    return parser.read(path)
