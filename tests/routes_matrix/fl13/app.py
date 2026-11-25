from flask import Flask
from flask.views import View

app = Flask(__name__, static_folder=None)


class IndexView(View):
    def dispatch_request(self):
        return "index"


app.add_url_rule("/", view_func=IndexView.as_view("i"))
