from fastapi import APIRouter, Depends, FastAPI
def authorize():
    return True
router = APIRouter(prefix="/api", dependencies=[Depends(authorize)])
@router.get("/items")
def items():
    return []
app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(router)
