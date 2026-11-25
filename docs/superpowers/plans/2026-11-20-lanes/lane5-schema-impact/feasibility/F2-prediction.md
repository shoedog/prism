# F2 prediction

This prediction precedes the F2 execution on public pair P at
`6744295fa888e0621fc9f9a8ace04f1a4daf8554` against its committed `docs/swagger.json`.

I predict 85–98% of counted sites will have a literal path after cutting the query,
and 75–95% will bind uniquely to a schema operation. I predict zero several-match
sites and expect every unique match to have an equal canonical path string. The
pinned schema is described as having no path parameters.

The principal expected sources of unbound sites are schema drift (a recoverable
method and path absent from the schema) and URL values passed into a wrapper or
built outside the local expression. I expect literal method names on most sites.
Query-only interpolation should not reduce path recoverability. The configured
barrel and legacy alias should resolve by file identity.
