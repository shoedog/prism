from contextlib import asynccontextmanager

from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


class EmptyRouteCatalog:
    routes = ()
    on_startup = ()
    on_shutdown = ()

    @asynccontextmanager
    async def lifespan_context(self, application):
        yield


catalog = EmptyRouteCatalog()
app.include_router(catalog)
