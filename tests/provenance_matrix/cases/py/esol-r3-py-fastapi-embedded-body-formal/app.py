from typing import Annotated
from fastapi import Body, FastAPI

app = FastAPI()

@app.post("/preview")
def preview(submitted: Annotated[str, Body(embed=True)]):
    return submitted
