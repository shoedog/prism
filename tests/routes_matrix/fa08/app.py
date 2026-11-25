from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


items = APIRouter()


@items.get("/items")
def list_items():
    return []


app.include_router(items, prefix="/v1")
app.include_router(items, prefix="/v2")
