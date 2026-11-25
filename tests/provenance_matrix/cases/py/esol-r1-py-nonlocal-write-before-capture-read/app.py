import os

def outer():
    mode = input()

    def inner():
        nonlocal mode
        mode = os.getenv("MODE")
        return mode

    return inner()
