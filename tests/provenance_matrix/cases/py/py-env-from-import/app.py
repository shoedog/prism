from os import getenv

def load():
    mode = getenv("MODE")
    return mode
