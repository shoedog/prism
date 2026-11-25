from django.http import HttpResponse
from django.urls import path
from django.views.decorators.http import require_http_methods


@require_http_methods(["PATCH"])
def edit(request):
    return HttpResponse("saved")


urlpatterns = [path("draft/edit/", edit)]
