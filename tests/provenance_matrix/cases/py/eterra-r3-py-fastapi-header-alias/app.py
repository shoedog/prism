from fastapi import FastAPI, Header
app = FastAPI()
@app.get('/')
def read(token: str = Header(alias='X-Token')):
    return token
