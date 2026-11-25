import os
edition = os.getenv("EDITION")
class Catalog:
    edition = input()
    by_char = [character for character in
               edition]
    repeated = [
        edition
        for _ in range(1)]
