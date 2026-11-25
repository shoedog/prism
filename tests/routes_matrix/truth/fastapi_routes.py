"""Trace FastAPI router declarations and copies made by include_router."""
from functools import wraps
import inspect


def install(trace, refusal, hand_required, normalize_methods, method_arguments):
    from fastapi import FastAPI
    from fastapi.routing import APIRoute, APIRouter, APIWebSocketRoute
    from starlette.routing import Mount

    routes = {}
    inclusion = None

    def registration(original, kind):
        signature = inspect.signature(original)

        @wraps(original)
        def wrapped(router, *args, **kwargs):
            arguments = signature.bind(router, *args, **kwargs).arguments
            path, endpoint = arguments["path"], arguments["endpoint"]
            origin = None
            if inclusion is not None and inclusion[0] is router:
                _, prefix, originals = inclusion
                if not originals:
                    raise refusal("unexpected FastAPI include_router copy")
                source = originals.pop(0)
                if source.endpoint is not endpoint or prefix + source.path != path:
                    raise refusal("FastAPI include_router copy changed path or callback identity")
                if id(source) not in routes:
                    raise refusal(f"untraced FastAPI source route: {source.path}")
                _, origin, registered_endpoint = routes[id(source)]
                if endpoint is not registered_endpoint:
                    raise refusal("FastAPI source route changed callback identity")
            call = trace.record(kind, {
                "path": path, "methods": method_arguments(arguments.get("methods")),
                "name": arguments.get("name"), "router_prefix": router.prefix,
            }, endpoint, origin, mounted=origin is not None)
            call["framework"] = "fastapi"
            before = len(router.routes)
            result = original(router, *args, **kwargs)
            if len(router.routes) != before + 1 or router.routes[-1].endpoint is not endpoint:
                raise refusal("expected one FastAPI route with the registered callback")
            route = router.routes[-1]
            routes[id(route)] = route, call, endpoint
            return result

        return wrapped

    original_include = APIRouter.include_router

    @wraps(original_include)
    def include(router, child, *, prefix="", **kwargs):
        nonlocal inclusion
        previous = inclusion
        originals = [route for route in child.routes if isinstance(route, (APIRoute, APIWebSocketRoute))]
        inclusion = router, prefix, originals
        try:
            result = original_include(router, child, prefix=prefix, **kwargs)
            if originals:
                raise refusal("missing FastAPI include_router copy")
            return result
        finally:
            inclusion = previous

    APIRouter.add_api_route = registration(APIRouter.add_api_route, "api_route")
    APIRouter.add_api_websocket_route = registration(APIRouter.add_api_websocket_route, "websocket")
    APIRouter.include_router = include

    def extract(app):
        served = []
        for route in app.routes:
            if isinstance(route, Mount):
                raise hand_required(f"untraced FastAPI mount: {route.path}")
            if id(route) not in routes:
                raise refusal(f"untraced FastAPI route: {route.path}")
            _, call, endpoint = routes[id(route)]
            if route.endpoint is not endpoint:
                raise refusal(f"FastAPI route changed callback identity: {route.path}")
            chain = [call["id"]]
            while call["mounted"]:
                call = trace.calls[call["origin_call_id"] - 1]
                chain.append(call["id"])
            kind = "websocket" if isinstance(route, APIWebSocketRoute) else "http"
            served.append({
                "kind": kind, "path": route.path, "endpoint": route.name,
                "declared_methods": normalize_methods(call["args"]["methods"]),
                "effective_methods": normalize_methods(route.methods) if kind == "http" else [],
                "call_ids": list(reversed(chain)),
            })
        return served

    return "fastapi", FastAPI, extract
