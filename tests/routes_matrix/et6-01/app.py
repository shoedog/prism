from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
router = APIRouter()
app.include_router(router, prefix="/v1")

@router.get("/items")
def list_items():
    return []

app.include_router(router, prefix="/v2")
