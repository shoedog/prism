from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.delete("/tokens/{token}")
def revoke_token(token: str):
    return {"ok": True}
