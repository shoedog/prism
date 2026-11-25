from fastapi import FastAPI

def create_app():
    app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)

    def install():
        @app.get("/status")
        def status():
            return {"ready": True}

    install()
    return app
