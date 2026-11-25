import requests


class Client:
    @staticmethod
    def fetch(u):
        return requests.get(u)
