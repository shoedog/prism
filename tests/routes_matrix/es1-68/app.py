from flask import Flask

app = Flask(__name__, static_folder=None)
section = "docs"

@app.get("/n\u006ftes")
def notes():
    return "notes"

@app.get("/items/" + "latest")
def latest():
    return "latest"

@app.get(f"/{section}")
def docs():
    return "docs"
