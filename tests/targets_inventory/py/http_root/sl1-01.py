import requests
def outer(requests):
    def middle():
        global requests
        def inner(u):
            return requests.get(u)
        return inner
    return middle
