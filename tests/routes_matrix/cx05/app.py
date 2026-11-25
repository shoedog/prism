from flask import Flask
from flask.views import MethodView

app = Flask(__name__, static_folder=None)


class Preferences(MethodView):
    def get(self):
        return {"theme": "light"}

    def patch(self):
        return "", 204


app.add_url_rule("/preferences", view_func=Preferences.as_view("preferences"))
