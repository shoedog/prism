from fastapi import APIRouter, FastAPI, WebSocket

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
feed = APIRouter(prefix="/feed")

async def stream(socket: WebSocket):
    await socket.accept()
    await socket.close()

def describe():
    return {"protocol": "websocket"}

feed.add_api_route("/status", describe, methods=["GET"])
feed.add_api_websocket_route(path="/status", endpoint=stream)
app.include_router(feed, prefix="/v2")
