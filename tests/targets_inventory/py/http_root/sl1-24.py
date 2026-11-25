import requests
def outer(u):
    def inner():
        return requests.get(u)
    class C:
        value = requests.get(u)
    x = = 1
