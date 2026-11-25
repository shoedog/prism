from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
child = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)

@child.get("/items")
def items():
    return []

app.mount("/api", child)
