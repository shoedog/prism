import urllib3


def fetch(u):
    return urllib3.request("GET", u)
