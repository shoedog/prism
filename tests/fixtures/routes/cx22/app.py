from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.patch("/draft")
def edit_draft():
    return {"ok": True}
