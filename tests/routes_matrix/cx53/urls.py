from django.urls import include, path, re_path

urlpatterns = [path("ops/", include("reports.urls"))]
