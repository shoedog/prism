from fastapi import APIRouter

router = APIRouter()


@router.post("/receipts")
def receive():
    return {"accepted": True}
