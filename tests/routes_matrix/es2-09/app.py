from flask import Flask

app = Flask(__name__, static_folder=None)

@app.get("/literal/:id/{draft}")
def literal():
    return "draft"

@app.get("/storage/<path:key>")
def storage(key):
    return key
