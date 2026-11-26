from flask import Flask

app = Flask(__name__, static_folder=None)

def status():
    return "ready"

for page in ("/imports", "/exports"):
    app.add_url_rule(page, view_func=status, methods=["GET"])
