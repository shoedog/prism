from flask import Flask, request
app = Flask(__name__)
@app.get("/")
def index():
    token = request.cookies.get("session")
    return token
