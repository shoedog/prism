from typing import Annotated
from fastapi import FastAPI, Cookie as CookieValue
app = FastAPI()

@app.get("/catalog")
async def show(preview: Annotated[str, CookieValue(alias="catalog-mode")]):
    return preview
