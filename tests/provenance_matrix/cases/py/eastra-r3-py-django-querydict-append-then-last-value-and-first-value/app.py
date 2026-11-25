from django.http import JsonResponse
from django.urls import path

def editions(incoming):
    if "edition" not in incoming.GET:
        return JsonResponse({})
    fields = incoming.GET.copy()
    fields.appendlist("edition", "preview")
    latest = fields["edition"]
    earliest = fields.getlist("edition")[0]
    return JsonResponse([latest, earliest], safe=False)

urlpatterns = [path("editions/", editions)]
