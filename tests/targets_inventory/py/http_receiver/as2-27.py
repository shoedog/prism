from httpx import Client
def fetch(seed: Client, u):
    return Client(base_url=seed
                  .get(u).url
                  ).get(u)
