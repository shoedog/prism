import httpx

def load(url):
    with httpx.Client() as client:
        response = client.get(url)
        return response
