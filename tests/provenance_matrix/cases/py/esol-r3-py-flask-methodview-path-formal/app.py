from flask import Flask
from flask.views import MethodView

app = Flask(__name__)

class Preview(MethodView):
    def get(self, label):
        return label

app.add_url_rule("/preview/<label>", view_func=Preview.as_view("preview"))
