from fastapi import FastAPI, Request
import pickle

app = FastAPI()

@app.post("/load")
async def load_config(request: Request):
    return pickle.loads(await request.body())
