def fetch_config(name):
    return name.startswith("show")

def show():
    state = fetch_config("show-mode")
    return state
