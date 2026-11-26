from flask import Flask

from legacy import *
from current import *

app = Flask(__name__, static_folder=None)
app.register_blueprint(blueprint, url_prefix="/ops")
