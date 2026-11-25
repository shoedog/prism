def configure(requester):
    requests = requester.return_value
    requests.post().json.return_value = {}
