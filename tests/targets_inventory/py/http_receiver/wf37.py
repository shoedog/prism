from urllib3 import PoolManager
def fetch(u, replacement):
    with PoolManager() as client:
        setattr(client, "urlopen", replacement)
        return client.urlopen("GET", u)
