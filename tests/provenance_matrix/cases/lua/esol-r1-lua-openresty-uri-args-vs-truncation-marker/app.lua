local args, failure = ngx.req.get_uri_args(20)
local label = args.label
ngx.say(label, failure)
