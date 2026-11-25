from flask import Flask

app = Flask(__name__, static_folder=None)


@app.get("/history")
@app.post("/history")
def history():
    return {"entries": []}
