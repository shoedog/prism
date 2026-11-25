from django.urls import include, path
urlpatterns = []
urlpatterns += [path("", include(("reports.urls", "reports"), namespace="reports"))]
