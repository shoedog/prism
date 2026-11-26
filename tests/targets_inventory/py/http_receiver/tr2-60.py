import requests
def outer():
    def make() -> requests.Session:
        return requests.Session()
    client = make()
    return client.get("u")
