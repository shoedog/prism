from django.urls import include, path

urlpatterns = [path("staff/", include("staff")), path("support/", include("support"))]
