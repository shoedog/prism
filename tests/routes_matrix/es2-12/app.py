from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
feed = APIRouter(prefix="/feed")

@feed.get("")
def read_feed():
    return []

app.include_router(feed, prefix="/v2")
