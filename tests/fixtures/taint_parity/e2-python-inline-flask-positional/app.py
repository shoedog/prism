from flask import Flask, request
import pickle
app = Flask(__name__)
@app.route("/")
def f():
    return pickle.loads(request.args.get("data")) # @SINK

def parity_anchor():
    t = acquire() # @SRC
