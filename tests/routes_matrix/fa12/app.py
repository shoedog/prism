from pathlib import Path

from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.mount("/s", StaticFiles(directory=Path(__file__).with_name("assets")))
