from django.http import HttpResponse
from django.urls import path


def schedule(request):
    return HttpResponse("schedule")


def submission(request):
    return HttpResponse("submission")


submission.should_append_slash = False

urlpatterns = [
    path("schedule/", schedule),
    path("submission/", submission),
]
