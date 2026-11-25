from urllib.request import urlopen


def fetch(u):
    return urlopen(u)
