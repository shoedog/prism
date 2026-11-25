from flask import Flask, request
app = Flask(__name__)

@app.post("/preview")
def preview():
    incoming = request.get_data(cache=False)
    exhausted = request.get_data(cache=False)
    return incoming, 200, {"X-Exhausted": str(len(exhausted))}
