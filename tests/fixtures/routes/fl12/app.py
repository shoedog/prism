from flask import Flask

app = Flask(__name__, static_folder=None)


app.add_url_rule("/", endpoint="i")
