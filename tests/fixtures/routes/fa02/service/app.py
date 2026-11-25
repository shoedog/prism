from fastapi import FastAPI

from . import items

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(items.router)
