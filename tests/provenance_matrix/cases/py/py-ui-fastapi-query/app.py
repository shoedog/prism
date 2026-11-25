from typing import Annotated
from fastapi import FastAPI, Query
app = FastAPI()

@app.get("/search")
def search(term: Annotated[str, Query()]):
    return term
