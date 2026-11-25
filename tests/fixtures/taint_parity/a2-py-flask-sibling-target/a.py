from flask import Flask, request
app = Flask(__name__)

def isolated():
    t = acquire()  # @SRC
    observe(t)

@app.route("/view")
def handler():
    name, value = request.args["term"], "fixed"
    return getattr(record, value)  # @SINK
