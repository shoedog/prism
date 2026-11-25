import os

def outer():
    mode = os.getenv("MODE")

    def inner():
        return mode

    return inner
