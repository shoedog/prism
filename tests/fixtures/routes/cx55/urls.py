from django.urls import include, path


PREFIX = "staff/"


urlpatterns = [path(PREFIX, include("reports.urls"))]
