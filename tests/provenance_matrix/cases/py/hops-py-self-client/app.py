import httpx

class Reader:
    def __init__(self):
        self.client = httpx.Client()

    def load(self, url):
        response = self.client.get(url)
        return response
