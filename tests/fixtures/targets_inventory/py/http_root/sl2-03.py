def f(u):
    import requests
    class C:
        nonlocal requests
        requests = None
    return requests.get(u)
