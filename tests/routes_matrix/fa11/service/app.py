from fastapi import FastAPI

from .routers import users as u

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(u.router)
