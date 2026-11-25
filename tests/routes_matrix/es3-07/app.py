from flask import Flask

app = Flask(__name__, static_folder=None)
saved = app.get('/inventory')


def inventory():
    return 'inventory'


saved(inventory)
