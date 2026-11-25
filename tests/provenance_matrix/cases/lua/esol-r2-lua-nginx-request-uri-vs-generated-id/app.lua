local function observe()
    local address = ngx.var.request_uri
    local trace_id = ngx.var.request_id
    return address, trace_id
end
