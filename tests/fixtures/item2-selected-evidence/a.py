def source(request):
    query = request.GET["q"]
    return query
