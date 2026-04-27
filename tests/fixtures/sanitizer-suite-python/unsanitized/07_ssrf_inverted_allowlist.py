from flask import Flask, request
from urllib.parse import urlparse
import requests

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
def fetch():
    url = request.args.get("url")
    parsed = urlparse(url)
    if parsed.hostname in ALLOWED_HOSTS:
        return "blocked"
    return requests.get(url)
