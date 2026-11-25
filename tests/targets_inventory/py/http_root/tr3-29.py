import requests
def f():
    def inner(
        x=requests.get("u"),
    ):
        return x
    return inner
