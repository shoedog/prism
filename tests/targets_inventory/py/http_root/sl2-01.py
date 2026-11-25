import requests
def outer():
    import httpx as requests
    class C:
        global requests
        value = requests.get("class")
        def fetch(self):
            return requests.get("method")
    return C
