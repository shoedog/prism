from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.head("/archive")
def archive_head():
    return {"ok": True}
