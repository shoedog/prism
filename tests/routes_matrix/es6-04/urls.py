from django.http import HttpResponse
from django.urls import path


def public_directory(request):
    return HttpResponse("public directory")


urlpatterns = [path("directory/", public_directory)]
