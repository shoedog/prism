from importlib import import_module
def fetch(u):
    requests = import_module("requests")
    return requests.get(u)
