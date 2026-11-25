from fastapi import FastAPI
from pydantic import BaseModel
app = FastAPI()
class Item(BaseModel):
    value: str
@app.post("/")
def f(item: Item):
    cursor.execute(item.value) # @SINK

def parity_anchor():
    t = acquire() # @SRC
