import configparser
def f():
    p = configparser.RawConfigParser()
    return p.read_file("x")
