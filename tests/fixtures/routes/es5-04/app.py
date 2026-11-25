from fastapi import APIRouter, FastAPI

app = FastAPI(root_path="/gateway", docs_url=None, redoc_url=None, openapi_url=None)
router = APIRouter()


def queue():
    return {"queued": 0}


router.add_api_route(endpoint=queue, include_in_schema=False, path="/queue", methods=["GET"])
app.include_router(router, prefix="/internal")
