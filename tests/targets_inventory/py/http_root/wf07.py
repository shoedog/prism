import urllib3
def fetch(u, replacement):
    vars(urllib3)["request"] = replacement
    return urllib3.request("GET", u)
