import requests


class Client:
    def method(self, u):
        def helper():
            return requests.get(u)

        return helper
