import requests


class Outer:
    class Inner:
        def m(self, u):
            return requests.get(u)
