from django.http import JsonResponse
from django.urls import path

def show(request, shelf, edition):
    return JsonResponse([shelf, edition], safe=False)

urlpatterns = [
    path("books/<str:shelf>/<str:edition>/", show, {"edition": "preview"}),
]
