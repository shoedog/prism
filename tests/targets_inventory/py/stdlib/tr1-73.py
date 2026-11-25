import configparser
def f():
    p = configparser.ConfigParser()
    return p.read_string("x")
