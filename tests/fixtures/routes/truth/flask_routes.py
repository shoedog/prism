"""Trace Flask declarations, deferred Blueprint registrations and URL rules."""
from functools import wraps
import inspect


def install(trace, refusal, hand_required, normalize_methods, method_arguments):
    from flask import Blueprint, Flask

    rules = {}
    deferred_origin = None
    blueprint_depth = 0

    def url_rule(original, blueprint):
        signature = inspect.signature(original)

        @wraps(original)
        def wrapped(receiver, *args, **kwargs):
            arguments = signature.bind(receiver, *args, **kwargs).arguments
            options = arguments.get("options", {})
            view = arguments.get("view_func")
            mounted = not blueprint and blueprint_depth > 0
            origin = None
            if mounted:
                if deferred_origin is None:
                    raise refusal("untraced Flask blueprint registration")
                origin, expected_view = deferred_origin
                if view is not expected_view:
                    raise refusal("Flask blueprint copy changed callback identity")

            call = trace.record("url_rule", {
                "rule": arguments["rule"], "endpoint": arguments.get("endpoint"),
                "methods": method_arguments(options.get("methods")),
            }, view, origin, mounted=mounted)
            call["framework"] = "flask"
            if blueprint:
                before = len(receiver.deferred_functions)
                result = original(receiver, *args, **kwargs)
                if len(receiver.deferred_functions) != before + 1:
                    raise refusal("expected one Flask blueprint deferred registration")
                deferred = receiver.deferred_functions[before]

                @wraps(deferred)
                def register_copy(state):
                    nonlocal deferred_origin
                    previous = deferred_origin
                    deferred_origin = call, view
                    try:
                        return deferred(state)
                    finally:
                        deferred_origin = previous

                receiver.deferred_functions[before] = register_copy
                return result

            before = {id(rule) for rule in receiver.url_map.iter_rules()}
            result = original(receiver, *args, **kwargs)
            added = [rule for rule in receiver.url_map.iter_rules() if id(rule) not in before]
            if len(added) != 1:
                raise refusal("expected one new Flask URL rule")
            rules[id(added[0])] = added[0], call, view
            return result

        return wrapped

    original_register = Blueprint.register

    @wraps(original_register)
    def register(blueprint, *args, **kwargs):
        nonlocal blueprint_depth
        blueprint_depth += 1
        try:
            return original_register(blueprint, *args, **kwargs)
        finally:
            blueprint_depth -= 1

    Flask.add_url_rule = url_rule(Flask.add_url_rule, False)
    Blueprint.add_url_rule = url_rule(Blueprint.add_url_rule, True)
    Blueprint.register = register

    def extract(app):
        if app.has_static_folder or any(bp.has_static_folder for bp in app.blueprints.values()):
            raise hand_required("Flask static routes must be disabled")
        served = []
        for rule in app.url_map.iter_rules():
            if id(rule) not in rules:
                raise refusal(f"untraced Flask rule: {rule.rule}")
            _, call, view = rules[id(rule)]
            actual_view = app.view_functions.get(rule.endpoint)
            if view is not None and actual_view is not view:
                raise refusal(f"Flask endpoint changed callback identity: {rule.endpoint}")
            origin_id = call.get("origin_call_id", call["id"])
            origin = trace.calls[origin_id - 1]
            served.append({
                "kind": "http", "path": rule.rule, "endpoint": rule.endpoint,
                "declared_methods": normalize_methods(origin["args"]["methods"]),
                "effective_methods": normalize_methods(rule.methods) if rule.methods is not None else "ANY",
                "call_ids": [origin_id, call["id"]] if call["mounted"] else [call["id"]],
                "runtime_callback": trace.callback(actual_view) if actual_view is not None else None,
            })
        return served

    return "flask", Flask, extract
