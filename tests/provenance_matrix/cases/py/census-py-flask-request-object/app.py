from flask import Flask, request
app = Flask(__name__)

@app.get("/preview")
def show():
    context = request
    return context
