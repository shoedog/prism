# Slice E — caddy interface-dispatch adjudication worklist (the 63 `fanout>0` sites)

> Source: `prism nav interface-manifest --repo ~/code/bench-repos/caddy` (caddy @ `77e9ce7404c4`), filtered to `fanout > 0`. Regenerate with that command + `jq '.sites[]|select(.fanout>0)'`. These are the prism *dispatch* sites the κ session adjudicates against the gopls oracle (Slice-E §5). 0 same-(file,line) collisions → line-keyed adjudication is unambiguous for caddy (Slice-E §2).

**63 dispatch sites** · by method: `ServeHTTP` 23 · `CaddyModule` 14 · `Adapt` 12 · `CertMagicStorage` 8 · `ConnectionState` 2 · `LoadConfig` 2 · `AcceptEncoding` 1 · `NewEncoder` 1

by receiver class: `typed_param` 26 · `type_assertion` 24 · `constructor_local` 13. (PR-2's `type_assertion` form contributes 24 of these.)

| # | file:line | method | fanout | receiver_class |
|---|---|---|---|---|
| 1 | `caddy.go:618` | `CaddyModule` | 121 | type_assertion |
| 2 | `caddyconfig/httpcaddyfile/httptype.go:301` | `CaddyModule` | 121 | type_assertion |
| 3 | `caddyconfig/httpcaddyfile/httptype.go:851` | `CaddyModule` | 121 | type_assertion |
| 4 | `caddyconfig/httpcaddyfile/httptype.go:865` | `CaddyModule` | 121 | type_assertion |
| 5 | `caddyconfig/httpcaddyfile/serveroptions.go:100` | `CaddyModule` | 121 | type_assertion |
| 6 | `caddyconfig/httpcaddyfile/serveroptions.go:120` | `CaddyModule` | 121 | type_assertion |
| 7 | `caddyconfig/httpcaddyfile/serveroptions.go:268` | `CaddyModule` | 121 | type_assertion |
| 8 | `caddyconfig/httpcaddyfile/tlsapp.go:180` | `CaddyModule` | 121 | type_assertion |
| 9 | `caddyconfig/httpcaddyfile/tlsapp.go:334` | `CaddyModule` | 121 | type_assertion |
| 10 | `caddyconfig/httpcaddyfile/tlsapp.go:480` | `CaddyModule` | 121 | type_assertion |
| 11 | `caddyconfig/httpcaddyfile/tlsapp.go:558` | `CaddyModule` | 121 | type_assertion |
| 12 | `logging.go:220` | `CaddyModule` | 121 | typed_param |
| 13 | `modules.go:139` | `CaddyModule` | 121 | typed_param |
| 14 | `modules/caddytls/ech.go:815` | `CaddyModule` | 121 | type_assertion |
| 15 | `modules/caddyhttp/routes.go:338` | `ServeHTTP` | 18 | typed_param |
| 16 | `modules/caddyhttp/encode/encode.go:194` | `AcceptEncoding` | 2 | typed_param |
| 17 | `modules/caddyhttp/encode/encode.go:206` | `NewEncoder` | 2 | typed_param |
| 18 | `caddyconfig/httpcaddyfile/builtins_test.go:85` | `Adapt` | 1 | constructor_local |
| 19 | `caddyconfig/httpcaddyfile/builtins_test.go:226` | `Adapt` | 1 | constructor_local |
| 20 | `caddyconfig/httpcaddyfile/builtins_test.go:289` | `Adapt` | 1 | constructor_local |
| 21 | `caddyconfig/httpcaddyfile/builtins_test.go:362` | `Adapt` | 1 | constructor_local |
| 22 | `caddyconfig/httpcaddyfile/httptype_test.go:70` | `Adapt` | 1 | constructor_local |
| 23 | `caddyconfig/httpcaddyfile/httptype_test.go:206` | `Adapt` | 1 | constructor_local |
| 24 | `caddyconfig/httpcaddyfile/httptype_test.go:226` | `Adapt` | 1 | constructor_local |
| 25 | `caddyconfig/httpcaddyfile/options_test.go:57` | `Adapt` | 1 | constructor_local |
| 26 | `caddyconfig/httpcaddyfile/options_test.go:124` | `Adapt` | 1 | constructor_local |
| 27 | `caddyconfig/httpcaddyfile/options_test.go:190` | `Adapt` | 1 | constructor_local |
| 28 | `caddyconfig/httpcaddyfile/pkiapp_test.go:38` | `Adapt` | 1 | constructor_local |
| 29 | `caddyconfig/httpcaddyfile/pkiapp_test.go:82` | `Adapt` | 1 | constructor_local |
| 30 | `caddy.go:546` | `CertMagicStorage` | 1 | type_assertion |
| 31 | `cmd/storagefuncs.go:90` | `CertMagicStorage` | 1 | type_assertion |
| 32 | `cmd/storagefuncs.go:162` | `CertMagicStorage` | 1 | type_assertion |
| 33 | `modules/caddypki/ca.go:116` | `CertMagicStorage` | 1 | type_assertion |
| 34 | `modules/caddytls/automation.go:202` | `CertMagicStorage` | 1 | type_assertion |
| 35 | `modules/caddytls/capools.go:412` | `CertMagicStorage` | 1 | type_assertion |
| 36 | `modules/caddytls/distributedstek/distributedstek.go:80` | `CertMagicStorage` | 1 | type_assertion |
| 37 | `modules/caddytls/leafstorageloader.go:63` | `CertMagicStorage` | 1 | type_assertion |
| 38 | `modules/caddyhttp/app.go:466` | `ConnectionState` | 1 | type_assertion |
| 39 | `modules/caddyhttp/http2listener.go:90` | `ConnectionState` | 1 | type_assertion |
| 40 | `caddy.go:643` | `LoadConfig` | 1 | type_assertion |
| 41 | `caddy.go:668` | `LoadConfig` | 1 | type_assertion |
| 42 | `metrics.go:56` | `ServeHTTP` | 1 | typed_param |
| 43 | `modules/caddyhttp/caddyauth/caddyauth.go:107` | `ServeHTTP` | 1 | typed_param |
| 44 | `modules/caddyhttp/encode/encode.go:181` | `ServeHTTP` | 1 | typed_param |
| 45 | `modules/caddyhttp/fileserver/staticfiles.go:724` | `ServeHTTP` | 1 | typed_param |
| 46 | `modules/caddyhttp/headers/headers.go:110` | `ServeHTTP` | 1 | typed_param |
| 47 | `modules/caddyhttp/headers/headers_test.go:366` | `ServeHTTP` | 1 | constructor_local |
| 48 | `modules/caddyhttp/intercept/intercept.go:161` | `ServeHTTP` | 1 | typed_param |
| 49 | `modules/caddyhttp/logging/logappend.go:95` | `ServeHTTP` | 1 | typed_param |
| 50 | `modules/caddyhttp/map/map.go:171` | `ServeHTTP` | 1 | typed_param |
| 51 | `modules/caddyhttp/push/handler.go:79` | `ServeHTTP` | 1 | typed_param |
| 52 | `modules/caddyhttp/push/handler.go:84` | `ServeHTTP` | 1 | typed_param |
| 53 | `modules/caddyhttp/push/handler.go:129` | `ServeHTTP` | 1 | typed_param |
| 54 | `modules/caddyhttp/requestbody/requestbody.go:81` | `ServeHTTP` | 1 | typed_param |
| 55 | `modules/caddyhttp/requestbody/requestbody.go:104` | `ServeHTTP` | 1 | typed_param |
| 56 | `modules/caddyhttp/reverseproxy/copyresponse.go:177` | `ServeHTTP` | 1 | typed_param |
| 57 | `modules/caddyhttp/rewrite/rewrite.go:139` | `ServeHTTP` | 1 | typed_param |
| 58 | `modules/caddyhttp/rewrite/rewrite.go:152` | `ServeHTTP` | 1 | typed_param |
| 59 | `modules/caddyhttp/server.go:529` | `ServeHTTP` | 1 | typed_param |
| 60 | `modules/caddyhttp/staticresp.go:254` | `ServeHTTP` | 1 | typed_param |
| 61 | `modules/caddyhttp/templates/templates.go:457` | `ServeHTTP` | 1 | typed_param |
| 62 | `modules/caddyhttp/vars.go:73` | `ServeHTTP` | 1 | typed_param |
| 63 | `modules/caddypki/acmeserver/acmeserver.go:250` | `ServeHTTP` | 1 | typed_param |
