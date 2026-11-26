"""Validate frozen P1d examples with the Draft 2020-12 implementation in jsonschema."""

import json
import sys

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


def first_error(schema, document, registry):
    Draft202012Validator.check_schema(schema)
    # Both schemas are bundled. Disallow fetching an external resource.
    validator = Draft202012Validator(
        schema, registry=registry, format_checker=Draft202012Validator.FORMAT_CHECKER
    )
    failure = next(validator.iter_errors(document), None)
    return failure.message if failure is not None else None


job = json.load(sys.stdin)
registry = Registry().with_resources(
    (schema["$id"], Resource.from_contents(schema)) for schema in (job["own"], job["opposite"])
)
control_error = first_error(job["own"], job["control"], registry)
own_instance_error = first_error(job["own"], job["instance"], registry)
error = first_error(job["opposite"], job["instance"], registry) if job["cross"] else own_instance_error
print(json.dumps({
    "control_error": control_error,
    "own_instance_error": own_instance_error if job["cross"] else None,
    "valid": error is None,
    "error": error,
}))
