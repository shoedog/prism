from flask import Flask, request
from urllib.parse import urlparse
import aiohttp

ALLOWED_HOSTS = {"example.com"}
app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    parsed = urlparse(url)
    if parsed.hostname not in ALLOWED_HOSTS:
        return "blocked"
    return await aiohttp.ClientSession().get(url)
