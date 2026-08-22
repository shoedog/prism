def lookup_view(request):
	q = request.GET["q"]
	cursor.execute(f"SELECT * FROM users WHERE name = '{q}'")
