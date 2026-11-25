from flask import Flask, request
app = Flask(__name__)

def isolated():
    t = acquire()  # @SRC
    observe(t)

def handler():
    value = request.args.get("term")
    return getattr(record, value)  # @SINK
app.add_url_rule("/view", view_func=handler)
