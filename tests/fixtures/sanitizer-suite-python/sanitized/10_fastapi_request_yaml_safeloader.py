from fastapi import FastAPI, Request
import yaml

app = FastAPI()

@app.post("/load")
async def load_config(request: Request):
    payload = await request.body()
    return yaml.load(payload, Loader=yaml.SafeLoader)
