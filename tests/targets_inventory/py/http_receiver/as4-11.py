from urllib3 import PoolManager
http = PoolManager()
def reset(transport):
    global http
    http = transport
def http_get(url):
    return http.request("GET", url)
