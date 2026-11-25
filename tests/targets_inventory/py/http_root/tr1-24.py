import requests
class C:
    @property
    def value(self):
        return requests.get("u")
