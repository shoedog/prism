import httpx
def run():
    with httpx.Client() as client:
        response = client.post('https://example.test', json={'a': 1})
        print(response)
