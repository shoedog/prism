import os
from flask import Flask, request

app = Flask(__name__)

@app.route('/run')
def handler():
    t = acquire() # @SRC
    cmd = request.args.get('cmd')
    os.system(cmd) # @SINK
    return 'ok'
