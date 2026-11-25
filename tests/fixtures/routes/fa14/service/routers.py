from fastapi import APIRouter

admin = APIRouter(tags=["admin"])
public = APIRouter(tags=["public"])


@admin.get("/admin/status")
def admin_status():
    return {"status": "ok"}


@public.get("/public/status")
def public_status():
    return {"status": "ok"}
