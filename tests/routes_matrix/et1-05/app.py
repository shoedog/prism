from fastapi import FastAPI
app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
def health():
    return {"ok": True}
app.router.add_api_route("/health", health, methods=["GET"])
