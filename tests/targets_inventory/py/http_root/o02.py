import urllib.request


def fetch(u):
    return urllib.request.urlopen(u)
