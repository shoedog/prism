from django.http import HttpResponse
from django.urls import path

def members(request):
    return HttpResponse("members")

routes = [path("members/", members)]
resolver = (routes, "accounts", "team")
