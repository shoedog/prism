from mylib import request

def read():
    value = request.args.get("mode")
    return value
