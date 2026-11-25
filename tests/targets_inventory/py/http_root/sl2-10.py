import requests
def f():
    return [requests
            .get("element") for requests in requests.get("iterable")]
