from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
EXTRA_METHOD = "PURGE"

@app.api_route("/queue", methods=["GET", EXTRA_METHOD])
def queue():
    return []
