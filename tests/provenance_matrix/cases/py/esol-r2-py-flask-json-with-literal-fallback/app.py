from flask import Flask, request
app = Flask(__name__)

@app.post("/catalog")
def submit():
    payload = request.get_json(silent=True) or {}
    return payload
