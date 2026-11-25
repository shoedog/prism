from typing import Annotated
from fastapi import Depends, FastAPI, Header

app = FastAPI()

def preview():
    return "preview"

@app.get("/book")
def show(label: Annotated[str, Header()], edition: Annotated[str, Depends(preview)]):
    return [label, edition]
