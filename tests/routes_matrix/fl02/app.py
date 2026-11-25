from flask import Flask

app = Flask(__name__, static_folder=None)


@app.get("/x")
def status():
    return {"status": "ok"}
