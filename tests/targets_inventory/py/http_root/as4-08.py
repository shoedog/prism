import requests
def factory(u, wrap):
    (requests
     .get(u))
    @wrap(0,,1)
    def inner():
        return requests.get(u)
    return inner
