import urllib3


def fetch(u):
    http = urllib3.PoolManager()
    return http.request("GET", u)
