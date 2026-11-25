def callback():
    return lambda requests=requests.get("default"): requests.get("body")
