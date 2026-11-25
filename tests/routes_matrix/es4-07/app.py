from fastapi import APIRouter, FastAPI
from starlette.responses import PlainTextResponse

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
router = APIRouter()

@router.route("/legacy", methods=["GET"])
async def legacy(request):
    return PlainTextResponse("legacy")

async def reload_config(request):
    return PlainTextResponse("reloaded")

app.include_router(router, prefix="/compat")
app.add_route("/reload", reload_config, methods=["POST"])
