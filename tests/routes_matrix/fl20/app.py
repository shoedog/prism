from flask import Flask

app = Flask(__name__, static_folder=None)


def replace(view):
    def replacement():
        return "replacement"

    return replacement


@app.get("/w")
@replace
def welcome():
    return "welcome"
