import os
from flask import Flask, request

app = Flask(__name__)

class Runner:
    def run(self, x):
        os.system(x) # @SINK

@app.route('/run')
def entry():
    t = acquire() # @SRC
    cmd = request.args.get('cmd')
    Runner().run(cmd)
    return 'ok'
