from configparser import ConfigParser, RawConfigParser
from typing import Optional
def make() -> Optional[RawConfigParser]:
    return ConfigParser()
def read(p):
    parser = make()
    return parser.read_file(p)
