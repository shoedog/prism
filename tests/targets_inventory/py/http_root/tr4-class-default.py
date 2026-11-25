def outer():
    class C:
        import requests
        def f(self, x=requests.get("u")):
            return requests.get("v")
    return C
