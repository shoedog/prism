from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.put("/template")
def replace_template():
    return {"ok": True}
