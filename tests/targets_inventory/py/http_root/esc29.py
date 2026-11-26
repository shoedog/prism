def fetch(u):
    import requests as transport
    result = transport.get(u)
    transport = {}
    return result
