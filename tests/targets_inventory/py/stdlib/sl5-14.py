import os
def f(key):
    environment = os.environ
    return environment.get(key)
