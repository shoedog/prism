import aiohttp
def fetch(u, replacement):
    alias = aiohttp
    alias.request = replacement
    return aiohttp.request("GET", u)
