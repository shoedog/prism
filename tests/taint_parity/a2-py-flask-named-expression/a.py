from flask import Flask, request
app = Flask(__name__)

def isolated():
    t = acquire()  # @SRC
    observe(t)

@app.route("/view")
def handler():
    value = "fixed"
    if (value := request.args["term"]):
        return getattr(record, value)  # @SINK
