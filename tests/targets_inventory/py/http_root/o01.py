from urllib import request


def fetch(u):
    return request.urlopen(u)
