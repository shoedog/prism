from fastapi import APIRouter
from .helper import present
router = APIRouter(prefix="/items")


@router.get("/{item_id}")
async def item(item_id: str):
    return present(item_id)


@router.websocket("/socket")
async def socket(ws):
    pass
