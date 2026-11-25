from django.http import HttpResponse
from django.urls import path
from django.utils.translation import gettext_lazy as _


def schedules(request):
    return HttpResponse("schedules")


urlpatterns = [path(_("schedules/"), schedules)]
