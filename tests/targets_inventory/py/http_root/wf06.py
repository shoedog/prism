import httpx
def fetch(u, replacement):
    httpx.__dict__["get"] = replacement
    return httpx.get(u)
