from flask import Flask, request
app = Flask(__name__)

def isolated():
    t = acquire()  # @SRC
    observe(t)

@app.route("/view")
def handler():
    value, *tail = "fixed", request.args["term"], 0
    return getattr(record, value)  # @SINK
