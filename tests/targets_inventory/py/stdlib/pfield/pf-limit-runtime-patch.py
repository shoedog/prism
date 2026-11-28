from configparser import ConfigParser

def replacement(self, name):
    return []

def patch(c):
    type(c).other = setattr(ConfigParser, "read", replacement)

def read_it():
    return ConfigParser().read("missing.ini")
