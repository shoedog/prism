from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.options("/export")
def export_options():
    return {"ok": True}
