import configparser
def f():
    p = configparser.RawConfigParser()
    return p.read_string("x")
