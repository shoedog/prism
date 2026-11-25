import requests
def f(value=None):
    client = requests.Session()
    match value:
        case [client]: pass
    return client.get("u")
