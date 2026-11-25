from flask import Blueprint, Flask

def public_routes():
    router = Blueprint("public", __name__)

    @router.get("/home")
    def home():
        return "public"

    return router

def admin_routes():
    router = Blueprint("admin", __name__)

    @router.get("/settings")
    def settings():
        return "admin"

    return router

def create_app():
    app = Flask(__name__, static_folder=None)
    public = public_routes()
    admin_routes()
    app.register_blueprint(public, url_prefix="/site")
    return app
