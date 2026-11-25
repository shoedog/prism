from flask import Flask

app = Flask(__name__, static_folder=None)

def replace(fn):
    def replacement():
        return "replacement"
    return replacement

@replace
@app.get("/welcome")
def welcome():
    return "welcome"
