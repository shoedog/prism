from fastapi import APIRouter

router = APIRouter(prefix="/warehouse")

@router.get("/counts")
def counts():
    return {}
