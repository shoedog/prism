import httpx
def run():
    response = httpx.get('https://example.org')
    url = httpx.URL('https://example.org')
    return response, url
