import os

def load():
    value = os.environ.get('MODE')
    count = len(os.environ)
    return value, count
