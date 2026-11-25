from django.conf.urls.i18n import i18n_patterns
from django.http import HttpResponse
from django.urls import path


def guide(request):
    return HttpResponse("guide")


urlpatterns = i18n_patterns(path("guide/", guide), prefix_default_language=False)
