from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
items = APIRouter(tags=["items"])
mirror = APIRouter(tags=["mirror"])

@items.get("/items")
def list_items():
    return []

@mirror.get("/health")
def health():
    return {"ready": True}

mirror.include_router(items, prefix="/first")
items.include_router(mirror, prefix="/mirror")
app.include_router(items, prefix="/api")
