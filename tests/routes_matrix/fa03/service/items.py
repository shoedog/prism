from fastapi import APIRouter

router = APIRouter(prefix="/items")


@router.get("/")
def items():
    return []
