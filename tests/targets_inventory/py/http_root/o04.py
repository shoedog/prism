import urllib


def fetch(u):
    return urllib.request.urlopen(u)
