from fastapi import FastAPI
from pydantic import BaseModel
app = FastAPI()
class Item(BaseModel):
    value: str
Model = Item

def isolated():
    t = acquire()  # @SRC

@app.post("/view")
def view(item: Model):
    return getattr(record, item.value)  # @SINK
