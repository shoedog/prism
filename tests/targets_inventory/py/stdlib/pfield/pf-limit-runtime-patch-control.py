from configparser import ConfigParser

def read_it():
    return ConfigParser().read("missing.ini")
