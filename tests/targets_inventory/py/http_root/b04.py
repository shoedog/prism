import requests


@deco(
    "flag",
)
def fetch(u):
    return requests.get(u)
