from fastapi import FastAPI, WebSocket

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.websocket("/ws")
async def updates(socket: WebSocket):
    await socket.accept()
    await socket.send_text("ready")
    await socket.close()
