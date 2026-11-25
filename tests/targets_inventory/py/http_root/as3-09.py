import requests
def fetch(render, u):
    return render(f"{{requests.get(u)}} {requests.get(u)}")
