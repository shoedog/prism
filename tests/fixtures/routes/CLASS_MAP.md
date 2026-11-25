# Routes census class map

Stage T coverage against the Stage M syntax ledger, `census.json` and `go_callback_shapes.md`. The ledger SHA-256 is `867a441dc2c127c5f8428b7b52fc1f45c13dc524a2df98c969cba18bb5966d58`. The ledger contains 5,607 sites.

A ledger class is one observed `(framework, kind, shape)` tuple. Handler and prefix values are independent dimensions within a framework. Hapi dimension counts also distinguish plugin objects, register calls, route calls and literal rows. Counts sum the syntax observations across corpora and test splits. A single site can contribute to multiple dimensions. The Go counts are non-test callback positions across the five corpora in the Go census, including Prometheus’s own router in that second look.

The framework name `route_table` comes from the census. Its fixture group is `w3-object-table`. The product framework label remains a Stage D decision. `d` marks provisional expectations. An `executed` label identifies a row for the truth runner. This fixture pass performed syntax and pin checks.

## Ledger shapes

| Framework | Kind | Shape | Sites | Non-test | Fixture rows |
| --- | --- | --- | --- | --- | --- |
| chi | mount | Route | 1 | 1 | `ch02`, `cx10` |
| django | mount | include | 88 | 1 | `cx53`, `cx54`, `cx55` |
| django | mount | path | 60 | 1 | `cx53`, `cx55` |
| django | mount | re_path | 16 | 0 | `cx54` |
| django | registration | path | 688 | 34 | `cx51`, `cx53`, `cx54`, `cx55`, `cx56`, `cx57`, `cx58`, `cx59`, `cx60` |
| django | registration | re_path | 144 | 13 | `cx52` |
| express | mount | use.mount | 73 | 41 | `ex19`, `ex20`, `ex21`, `ex22`, `ex23`, `ex24`, `ex25`, `ex26`, `ex29`, `ex30`, `cx01`, `cx03` |
| express | registration | builder.all | 6 | 0 | `cx11` |
| express | registration | builder.get | 4 | 0 | `ex10`, `ex28` |
| express | registration | builder.post | 2 | 0 | `ex10`, `ex28` |
| express | registration | use.middleware | 695 | 124 | `ex16`, `cx04` |
| express | registration | verb.all | 7 | 1 | `ex11` |
| express | registration | verb.bind | 2 | 0 | `cx12` |
| express | registration | verb.delete | 56 | 54 | `cx13` |
| express | registration | verb.get | 339 | 170 | `ex01`, `ex02`, `ex03`, `ex04`, `ex05`, `ex06`, `ex07`, `ex08`, `ex09`, `ex12`, `ex13`, `ex14`, `ex15`, `ex18`, `ex19`, `ex20`, `ex21`, `ex22`, `ex23`, `ex24`, `ex25`, `ex26`, `ex27`, `ex29`, `ex30`, `cx01`, `cx03`, `cx18` |
| express | registration | verb.head | 2 | 1 | `ex17` |
| express | registration | verb.options | 2 | 1 | `ex17` |
| express | registration | verb.patch | 56 | 56 | `cx14` |
| express | registration | verb.post | 133 | 115 | `cx15` |
| express | registration | verb.put | 7 | 3 | `cx16` |
| express | registration | verb.search | 23 | 23 | `cx17` |
| fastapi | mount | include_router | 146 | 7 | `fa02`, `fa03`, `fa08`, `fa09`, `fa10`, `fa11`, `fa13`, `fa14`, `cx23` |
| fastapi | registration | add_api_route | 5 | 3 | `fa05` |
| fastapi | registration | decorator.api_route | 2 | 0 | `fa04` |
| fastapi | registration | decorator.delete | 6 | 4 | `cx19` |
| fastapi | registration | decorator.get | 944 | 299 | `fa01`, `fa02`, `fa03`, `fa07`, `fa08`, `fa09`, `fa10`, `fa11`, `fa14`, `cx06` |
| fastapi | registration | decorator.head | 1 | 0 | `cx20` |
| fastapi | registration | decorator.options | 1 | 0 | `cx21` |
| fastapi | registration | decorator.patch | 4 | 3 | `cx22` |
| fastapi | registration | decorator.post | 301 | 92 | `cx23` |
| fastapi | registration | decorator.put | 39 | 36 | `cx24` |
| fastapi | registration | decorator.trace | 1 | 0 | `cx25` |
| fastapi | registration | decorator.websocket | 29 | 5 | `fa06` |
| flask | mount | register_blueprint | 74 | 3 | `fl05`, `fl06`, `fl07`, `fl08`, `fl09`, `fl10`, `fl16`, `cx02` |
| flask | registration | add_url_rule | 48 | 6 | `fl11`, `fl12`, `fl13`, `cx05`, `cx26`, `cx27`, `cx28` |
| flask | registration | decorator.get | 14 | 1 | `fl02`, `fl04`, `fl10`, `fl16`, `fl18`, `fl19`, `fl20`, `fl21`, `cx02` |
| flask | registration | decorator.post | 6 | 3 | `fl04` |
| flask | registration | decorator.route | 285 | 13 | `fl01`, `fl03`, `fl05`, `fl06`, `fl07`, `fl08`, `fl09`, `fl14`, `fl15`, `fl17` |
| gin | mount | Group | 28 | 13 | `gn02`, `gn04`, `gn05`, `gn06`, `gn09`, `gn10`, `gm05` |
| gin | registration | Any | 8 | 1 | `gn07` |
| gin | registration | DELETE | 15 | 9 | `cx29` |
| gin | registration | GET | 300 | 138 | `gn01`, `gn03`, `gn04`, `gn05`, `gn06`, `gn08`, `gn09`, `gn10`, `gm01`, `gm02`, `gm03`, `gm05`, `gm06`, `gm07`, `gm08`, `gm09`, `gm10`, `gm11`, `gm12`, `gm13`, `gm14`, `gm15`, `gm16`, `gm17`, `gm18`, `cx35`, `cx36`, `cx37`, `cx38`, `cx39`, `cx40`, `cx41`, `cx42`, `cx43` |
| gin | registration | HEAD | 8 | 3 | `cx30` |
| gin | registration | Handle | 9 | 1 | `gn07` |
| gin | registration | OPTIONS | 6 | 1 | `cx31` |
| gin | registration | PATCH | 7 | 1 | `cx32` |
| gin | registration | POST | 55 | 38 | `gn02` |
| gin | registration | PUT | 62 | 53 | `cx33` |
| hapi | mount | server.register | 81 | 6 | `hp01`, `hp02`, `hp03`, `hp04`, `hp05`, `hp06`, `hp07`, `hp08`, `hp09`, `hp10`, `hp11`, `hp14`, `hp17`, `hp18`, `hp20`, `hp21`, `hp22`, `cx07`, `cx08`, `cx44`, `cx45`, `cx46` |
| hapi | registration | route_row.array | 36 | 29 | `hp01`, `hp03`, `hp07`, `hp10`, `hp11`, `hp12`, `hp20`, `cx48` |
| hapi | registration | route_row.single | 21 | 11 | `hp02`, `hp04`, `hp05`, `hp06`, `hp08`, `hp09`, `hp14`, `hp15`, `hp16`, `hp17`, `hp18`, `hp21`, `hp22`, `cx07`, `cx08`, `cx44`, `cx45`, `cx46`, `cx49` |
| nestjs | mount | decorator.Controller | 56 | 56 | `cx61`, `cx62`, `cx63`, `cx64`, `cx65`, `cx66` |
| nestjs | registration | decorator.Delete | 28 | 28 | `cx62` |
| nestjs | registration | decorator.Get | 108 | 108 | `cx61`, `cx66` |
| nestjs | registration | decorator.Patch | 18 | 18 | `cx63` |
| nestjs | registration | decorator.Post | 85 | 85 | `cx64` |
| nestjs | registration | decorator.Put | 2 | 2 | `cx65` |
| net/http | registration | Handle.dynamic_pattern | 4 | 4 | `cx34` |
| net/http | registration | Handle.method_pattern | 16 | 16 | `cx09` |
| net/http | registration | Handle.plain_pattern | 12 | 6 | `nh03` |
| net/http | registration | HandleFunc.dynamic_pattern | 3 | 0 | `nh07` |
| net/http | registration | HandleFunc.method_pattern | 155 | 153 | `nh01`, `nh04`, `gm04`, `gm19` |
| net/http | registration | HandleFunc.plain_pattern | 24 | 7 | `nh02`, `nh05`, `nh06`, `nh08` |
| route_table | registration | array_row | 150 | 142 | `ol01`, `ol02`, `ol03`, `ol04`, `ol05`, `cx50` |

## Census dimensions

| Framework | Source | Dimension | Value | Observations | Fixture rows |
| --- | --- | --- | --- | --- | --- |
| django | - | handlers | imported | 115 | `cx56` |
| django | - | handlers | inline | 31 | `cx57` |
| django | - | handlers | member | 337 | `cx58` |
| django | - | handlers | named | 60 | `cx51`, `cx52` |
| django | - | handlers | other | 12 | `cx59` |
| django | - | handlers | wrapped | 277 | `cx60` |
| django | - | prefix_source | literal_at_mount | 72 | `cx53`, `cx54` |
| django | - | prefix_source | none | 920 | `cx51`, `cx52` |
| django | - | prefix_source | variable | 4 | `cx55` |
| express | - | handlers | imported | 321 | `ex07`, `ex27` |
| express | - | handlers | inline | 781 | `ex02` |
| express | - | handlers | member | 10 | `ex06` |
| express | - | handlers | named | 185 | `ex01`, `ex09` |
| express | - | handlers | other | 63 | `cx18` |
| express | - | handlers | wrapped | 468 | `ex03`, `ex04`, `ex05`, `ex08`, `ex09` |
| express | - | lookalike_possible | false | 1171 | `ex01`, `ex19` |
| express | - | lookalike_possible | true | 236 | `ex14` |
| express | - | prefix_source | literal_at_mount | 70 | `ex19`, `ex24`, `ex25` |
| express | - | prefix_source | literal_at_mount_in_another_file | 307 | `ex20`, `ex21`, `ex22`, `cx01` |
| express | - | prefix_source | none | 1027 | `ex01`, `ex16` |
| express | - | prefix_source | variable | 3 | `ex23` |
| fastapi | - | handlers | named | 1333 | `fa01`, `fa05`, `cx06` |
| fastapi | - | prefix_source | literal_at_mount | 89 | `fa08`, `fa09` |
| fastapi | - | prefix_source | literal_at_mount_in_another_file | 3 | `fa03`, `cx23` |
| fastapi | - | prefix_source | literal_on_router | 17 | `fa02` |
| fastapi | - | prefix_source | none | 1369 | `fa01`, `fa11` |
| fastapi | - | prefix_source | variable | 1 | `fa10` |
| flask | - | handlers | inline | 2 | `cx26` |
| flask | - | handlers | member | 2 | `cx27` |
| flask | - | handlers | named | 318 | `fl02`, `fl11` |
| flask | - | handlers | other | 13 | `fl12` |
| flask | - | handlers | wrapped | 18 | `fl13`, `cx05`, `cx28` |
| flask | - | prefix_source | literal_at_mount | 32 | `fl06`, `fl07`, `fl09` |
| flask | - | prefix_source | literal_on_router | 9 | `fl05`, `fl10`, `cx02` |
| flask | - | prefix_source | none | 384 | `fl01`, `fl05` |
| flask | - | prefix_source | variable | 2 | `fl08` |
| hapi | plugin_object | export_form | exports.plugin | 18 | `hp04` |
| hapi | plugin_object | export_form | local_or_factory | 13 | `hp01`, `cx07` |
| hapi | plugin_object | export_form | module.exports | 30 | `hp05`, `cx44`, `cx45` |
| hapi | plugin_object | plugin_shape | name+pkg+register | 15 | `cx44` |
| hapi | plugin_object | plugin_shape | name+register | 45 | `hp01`, `hp04`, `hp05` |
| hapi | plugin_object | plugin_shape | pkg+register | 1 | `cx45` |
| hapi | plugin_object | register_form | call/wrapper | 1 | `cx46` |
| hapi | plugin_object | register_form | inline function | 60 | `hp01` |
| hapi | route_row | argument_form | array | 36 | `hp01`, `cx48` |
| hapi | route_row | argument_form | single | 21 | `hp02`, `cx49` |
| hapi | route_row | handler_locations | config.handler | 5 | `hp01`, `cx49` |
| hapi | route_row | handler_locations | handler | 15 | `hp01`, `hp11` |
| hapi | route_row | handler_locations | missing | 24 | `hp20` |
| hapi | route_row | handler_locations | options.handler | 13 | `hp02` |
| hapi | route_row | handler_spelling | inline function | 28 | `hp11`, `cx49` |
| hapi | route_row | handler_spelling | member | 5 | `hp11` |
| hapi | route_row | handler_spelling | missing | 24 | `hp20` |
| hapi | route_row | handlers | inline | 28 | `hp11`, `cx49` |
| hapi | route_row | handlers | member | 5 | `hp11` |
| hapi | route_row | handlers | none | 24 | `hp20` |
| hapi | route_row | prefix_source | none | 57 | `hp01`, `hp02`, `cx49` |
| hapi | server.register | argument_form | array | 15 | `cx08` |
| hapi | server.register | argument_form | identifier | 1 | `hp01`, `hp02`, `hp06` |
| hapi | server.register | argument_form | object with plugin | 65 | `hp05`, `cx08` |
| hapi | server.register | prefix_source | literal_at_mount | 2 | `hp05`, `cx08` |
| hapi | server.register | prefix_source | none | 100 | `hp01`, `hp02`, `cx08` |
| hapi | server.register | routes_prefix | literal | 2 | `hp03`, `hp05`, `cx08` |
| hapi | server.register | routes_prefix | none | 100 | `hp01`, `hp02`, `cx08` |
| hapi | server.route | argument_form | array | 26 | `hp01`, `hp11`, `cx48` |
| hapi | server.route | argument_form | call | 1 | `cx47` |
| hapi | server.route | argument_form | single | 21 | `hp02`, `cx49` |
| hapi | server.route | inside_plugin_register | false | 11 | `cx49` |
| hapi | server.route | inside_plugin_register | true | 37 | `hp01`, `hp02` |
| hapi | server.route | nonliteral_rows | call/wrapper | 134 | `cx47`, `cx48` |
| hapi | server.route | plugin_reached | false | 48 | `hp16`, `hp22`, `cx07` |
| nestjs | - | handlers | named | 241 | `cx61`, `cx62`, `cx63`, `cx64`, `cx65`, `cx66` |
| nestjs | - | prefix_source | literal_on_router | 53 | `cx61`, `cx62`, `cx63`, `cx64`, `cx65` |
| nestjs | - | prefix_source | none | 244 | `cx66` |
| route_table | - | handlers | inline | 143 | `ol03` |
| route_table | - | handlers | member | 2 | `cx50` |
| route_table | - | handlers | named | 5 | `ol01`, `ol02`, `ol05` |
| route_table | - | prefix_source | none | 150 | `ol01`, `ol05` |
| route_table | - | reason | table_consumer_unproven | 150 | `ol05` |

## Go callback shapes

| Shape | Non-test positions | Fixture rows |
| --- | --- | --- |
| Method value on a one-hop field of the enclosing receiver | 211 | `gn03`, `gm01`, `gm02`, `gm07` |
| Method value on a two-hop field of the enclosing receiver | 0 | `cx35` |
| Method value on a local bound once to a field chain | 6 | `gm03` |
| Method value on a composite-literal local | 155 | `nh01`, `gm04` |
| Method value on a constructor-initialised local | 0 | `cx36` |
| Method value on another call-initialised local | 0 | `cx37` |
| Method value on the enclosing receiver | 3 | `cx38` |
| Method value on a typed parameter | 0 | `gm05`, `gm18` |
| Method value on another local or parameter | 0 | `cx39` |
| Method value on a one-hop field of a parameter, local or package var | 0 | `gm18` |
| Method value on a two-hop field of a parameter, local or package var | 0 | `cx40` |
| Method value on a package var | 0 | `gm06` |
| Method value on an interface-typed field | 0 | `gm09` |
| Method value on an interface-typed parameter | 0 | `gm10` |
| Method value on another interface-typed receiver | 1 | `cx41` |
| Method value on a call result | 1 | `gm13` |
| Method value on a receiver with three or more field hops | 0 | `gm13` |
| Method value on another receiver expression | 0 | `gm13` |
| Wrapper with a method-value argument | 64 | `gm19` |
| Bare function | 13 | `gn01`, `nh04`, `ch01` |
| Function variable or function-typed field | 23 | `gm14`, `cx42` |
| Forwarding function literal | 3 | `cx43` |
| Other function literal | 13 | `gn02`, `nh02` |
| Other wrapper call or conversion | 14 | `gn08`, `nh08`, `cx09` |
| Package function | 3 | `gm15` |
| Method expression | 0 | `gm15` |
| Other declared handler value | 5 | `nh03` |
| Unclassified callback | 0 | `out_of_scope` |

## Additional contract shapes

| Shape | Fixture rows |
| --- | --- |
| Express router index barrel | `cx01` |
| Express nested router.use with an array of path alternatives | `cx03` |
| Express app.use(path, fn) middleware | `cx04` |
| Flask Blueprint constructed in a package initializer | `cx02` |
| Flask MethodView dispatch | `cx05` |
| FastAPI Depends on a route | `cx06` |
| Gin root, no group prefix | `gn01` |
| Gin literal Group prefix | `gn02`, `gn04`, `gn10` |
| Gin configuration-derived Group prefix | `gn06` |
| net/http default mux, no prefix | `nh02` |
| net/http explicit mux, no prefix | `nh04` |
| net/http served routes through StripPrefix | `nh08` |
| net/http Handle with an http.HandlerFunc conversion | `cx09` |
| chi root and With, no prefix | `ch01`, `ch04` |
| chi factory-built child mount | `ch03` |
| chi nested Route closures | `cx10` |
| Hapi factory-built plugin list | `cx07` |
| Hapi plugin registration through a loop | `hp22` |
| Hapi single route object on a registered plugin parameter | `hp02` |
| Hapi inline register method | `hp02`, `cx08` |
| Hapi named or shorthand register identifier | `hp04`, `hp05` |
| Hapi ESM named and default plugin export | `hp06` |
| Hapi legacy register.attributes export | `hp21` |
| Hapi one-level imported route array | `hp19` |
| Hapi handler object, wrapper and pre-hook position | `hp11` |
| Hapi duplicate handler locations | `hp20` |
| Hapi variable register prefix | `hp10` |
| Hapi nested or repeated plugin mounts | `hp08`, `hp09` |

## Seed-only controls

These seed classes cover negative controls and design axes with no separate census bucket.

| Framework or group | Class | Fixture row | What it exercises |
| --- | --- | --- | --- |
| core | `no_framework` | `e01` | The inventory module imports no web framework and registers no routes or mounts, so all summary counts are zero. |
| core | `framework_without_registration` | `e02` | The Flask app disables the implicit static rule and declares no routes or mounts, so all summary counts are zero. |
| w1-python | `static_files_mount` | `fa12` | FastAPI is expected to add a Starlette ASGI mount. The provisional route and mount listings are empty pending Stage D's decision on StaticFiles scope. |
| w3-hapi | `fastify_lookalike_plugin` | `hp13` | DESIGN 11.2 What is refused. The imported Fastify instance passes a Fastify parameter and uses url, so the Hapi listing contains no registration. |
| w3-object-table | `unused_exported_table` | `ol06` | DESIGN 5.3. The file exports the table as data and no consumer receives it. |
| w3-object-table | `client_request_descriptor` | `ol07` | DESIGN 5.3. The method, path and handler describe an outbound HTTP request, with no server registration. |

## Coverage

The four tables enumerate **199 classes**. The **171 census classes** include 64 ledger shapes, 79 dimension values and 28 Go callback shapes. The additional table tracks 28 contract shapes. The seed rows covered 126 classes. The census rows close all 72 in-scope gaps. The 203 seed and census rows map to a class, including 6 seed-only controls. The enumeration rows (`et*`, `es*`, added later by the terra and sol tiers; 217 rows in `rows.jsonl`, each with its own `class` string) are not part of this census map: they are variant rows found by the enumeration tiers, not census classes. The counts of covered classes exclude those seed-only controls.

No in-scope classes remain uncovered.

### Classes uncovered by the seed rows

| Class | Added fixture rows |
| --- | --- |
| django / mount / include / 88 | `cx53`, `cx54`, `cx55` |
| django / mount / path / 60 | `cx53`, `cx55` |
| django / mount / re_path / 16 | `cx54` |
| django / registration / path / 688 | `cx51`, `cx53`, `cx54`, `cx55`, `cx56`, `cx57`, `cx58`, `cx59`, `cx60` |
| django / registration / re_path / 144 | `cx52` |
| express / registration / builder.all / 6 | `cx11` |
| express / registration / verb.bind / 2 | `cx12` |
| express / registration / verb.delete / 56 | `cx13` |
| express / registration / verb.patch / 56 | `cx14` |
| express / registration / verb.post / 133 | `cx15` |
| express / registration / verb.put / 7 | `cx16` |
| express / registration / verb.search / 23 | `cx17` |
| fastapi / registration / decorator.delete / 6 | `cx19` |
| fastapi / registration / decorator.head / 1 | `cx20` |
| fastapi / registration / decorator.options / 1 | `cx21` |
| fastapi / registration / decorator.patch / 4 | `cx22` |
| fastapi / registration / decorator.post / 301 | `cx23` |
| fastapi / registration / decorator.put / 39 | `cx24` |
| fastapi / registration / decorator.trace / 1 | `cx25` |
| gin / registration / DELETE / 15 | `cx29` |
| gin / registration / HEAD / 8 | `cx30` |
| gin / registration / OPTIONS / 6 | `cx31` |
| gin / registration / PATCH / 7 | `cx32` |
| gin / registration / PUT / 62 | `cx33` |
| nestjs / mount / decorator.Controller / 56 | `cx61`, `cx62`, `cx63`, `cx64`, `cx65`, `cx66` |
| nestjs / registration / decorator.Delete / 28 | `cx62` |
| nestjs / registration / decorator.Get / 108 | `cx61`, `cx66` |
| nestjs / registration / decorator.Patch / 18 | `cx63` |
| nestjs / registration / decorator.Post / 85 | `cx64` |
| nestjs / registration / decorator.Put / 2 | `cx65` |
| net/http / registration / Handle.dynamic_pattern / 4 | `cx34` |
| net/http / registration / Handle.method_pattern / 16 | `cx09` |
| django / - / handlers / imported | `cx56` |
| django / - / handlers / inline | `cx57` |
| django / - / handlers / member | `cx58` |
| django / - / handlers / named | `cx51`, `cx52` |
| django / - / handlers / other | `cx59` |
| django / - / handlers / wrapped | `cx60` |
| django / - / prefix_source / literal_at_mount | `cx53`, `cx54` |
| django / - / prefix_source / none | `cx51`, `cx52` |
| django / - / prefix_source / variable | `cx55` |
| express / - / handlers / other | `cx18` |
| flask / - / handlers / inline | `cx26` |
| flask / - / handlers / member | `cx27` |
| hapi / plugin_object / plugin_shape / name+pkg+register | `cx44` |
| hapi / plugin_object / plugin_shape / pkg+register | `cx45` |
| hapi / plugin_object / register_form / call/wrapper | `cx46` |
| hapi / server.register / argument_form / array | `cx08` |
| hapi / server.route / argument_form / call | `cx47` |
| hapi / server.route / inside_plugin_register / false | `cx49` |
| hapi / server.route / nonliteral_rows / call/wrapper | `cx47`, `cx48` |
| nestjs / - / handlers / named | `cx61`, `cx62`, `cx63`, `cx64`, `cx65`, `cx66` |
| nestjs / - / prefix_source / literal_on_router | `cx61`, `cx62`, `cx63`, `cx64`, `cx65` |
| nestjs / - / prefix_source / none | `cx66` |
| route_table / - / handlers / member | `cx50` |
| Method value on a two-hop field of the enclosing receiver | `cx35` |
| Method value on a constructor-initialised local | `cx36` |
| Method value on another call-initialised local | `cx37` |
| Method value on the enclosing receiver | `cx38` |
| Method value on another local or parameter | `cx39` |
| Method value on a two-hop field of a parameter, local or package var | `cx40` |
| Method value on another interface-typed receiver | `cx41` |
| Forwarding function literal | `cx43` |
| Express router index barrel | `cx01` |
| Express nested router.use with an array of path alternatives | `cx03` |
| Express app.use(path, fn) middleware | `cx04` |
| Flask Blueprint constructed in a package initializer | `cx02` |
| Flask MethodView dispatch | `cx05` |
| FastAPI Depends on a route | `cx06` |
| net/http Handle with an http.HandlerFunc conversion | `cx09` |
| chi nested Route closures | `cx10` |
| Hapi factory-built plugin list | `cx07` |

### Out of scope

- **Go unclassified callback** had zero positions and names no source shape for a built rule to exercise.

Fastify, Koa and DRF had no admitted census shapes. The existing Fastify lookalike is a Hapi negative control.

### Seed plan and design notes

- PLAN T-1 anticipates two exposures for an Express array mount. The pinned Express runner accepts literal-prefix layers and the array is one alternation layer. The existing `ex30` and the nested `cx03` use hand truth and leave the mount projection provisional.
- DESIGN 5.2 leaves a re-export chain unproven. The index-barrel fixture `cx01` requires a suffix and an unproven mount child even though Express serves the composed full path.
- PLAN T-1 shows `gn04` passing `r.Group("/api")` inline. DESIGN 5.3 specifies passing the group variable itself, which the existing fixture does.
- The Hapi loop and single-object plugin cases are already `hp22` and `hp02`. The new Hapi cases exercise a factory-built argument, a literal register array and other census gaps.
- The Django and NestJS adapters were admitted at Stage M. The full runtime URL shape guides the hand fixtures, with adapter choices such as regex-prefix projection and controller identity left for Stage D.
