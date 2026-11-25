import requests
def f(value=None):
    client = requests.Session()
    def client(): pass
    return client.get("u")
