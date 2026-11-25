from django.http import HttpResponse
from django.urls import include, path

def members(request, org):
    return HttpResponse(org)

urlpatterns = [path("org/<slug:org>/", include([path("members/", members)]))]
