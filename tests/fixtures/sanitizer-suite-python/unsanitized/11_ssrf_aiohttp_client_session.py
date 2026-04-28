from flask import Flask, request
import aiohttp

app = Flask(__name__)

@app.route("/fetch")
async def fetch():
    url = request.args.get("url")
    async with aiohttp.ClientSession() as session:
        return await session.get(url)
