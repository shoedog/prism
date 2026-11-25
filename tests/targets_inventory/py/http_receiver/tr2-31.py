import urllib3
def f():
    client = urllib3.PoolManager()
    return client.urlopen("https://example.test")
