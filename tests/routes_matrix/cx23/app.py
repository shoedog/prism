from fastapi import FastAPI
from intake import router

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(router, prefix="/warehouse")
