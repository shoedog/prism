from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.trace("/echo")
def trace_echo():
    return {"ok": True}
