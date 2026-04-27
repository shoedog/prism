from flask import Flask, request
import requests

app = Flask(__name__)

@app.route("/fetch")
def fetch():
    url = request.args.get("url")
    return requests.get(url)
