from fastapi import FastAPI

from ..features.warehouse import router as stock

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(stock, prefix="/ops")
