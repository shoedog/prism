from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
router = APIRouter(prefix="/reports")


def daily():
    return {"items": []}


router.add_api_route("/daily", daily, methods=["GET"])
app.include_router(router, prefix="/first")
router.add_api_route("/daily", daily, methods=["GET"])
app.include_router(router, prefix="/second")
