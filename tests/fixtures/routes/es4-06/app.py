from flask import Flask

app = Flask(__name__, static_folder=None)

@app.put("/preferences")
def replace():
    return "replaced"

@app.patch("/preferences")
def update():
    return "updated"

@app.delete("/preferences")
def remove():
    return "removed"
