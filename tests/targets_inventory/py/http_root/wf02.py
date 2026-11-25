import urllib.request as transport
def fetch(u, replacement):
    setattr(transport, "urlopen", replacement)
    return transport.urlopen(u)
