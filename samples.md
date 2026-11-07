### Django e8cff292: 40 sampled removed edges (seed 630; 30 stratified + 10 from top-20)
All 40 judged RIGHT (reading the source: dominating write on every path, or write-only use line, or alias twin before the real write). Loop cases were hand-read (nested loops re-enter at block top).

| kind | fn | path | def | use |
|---|---|---|---|---|
| strat | test_invalid_generic_ip_raises_error@157 | giptm | 164: `giptm = GenericIPAddressTestModel(generic_ip=lazy(lambd` | 162: `giptm = GenericIPAddressTestModel(generic_ip=1)` |
| strat | get@660 | limit | 678: `limit = MAX_GET_RESULTS` | 673: `limit = None` |
| strat | test_relatedfieldlistfilter_reverse_relationships@809 | filterspec | 855: `filterspec = changelist.get_filters(request)[0][1]` | 871: `filterspec.choices(changelist), "display", self.django_` |
| strat | test_intersection@403 | b | 406: `a = OGRGeometry(self.geometries.topology_geoms[i].wkt_a` | 409: `self.assertTrue(a.intersects(b))` |
| strat | test_12_arithmetic@429 | pl | 450: `pl *= n` | 477: `self.assertGreater(ul, pl, "cmp for gt self")` |
| strat | test_rename_m2m_through_model@1230 | Rider | 1291: `Rider = project_state.apps.get_model(app_label, "Rider"` | 1280: `rider = Rider.objects.create()` |
| strat | test_relatedfieldlistfilter_reverse_relationships@809 | changelist | 865: `changelist = modeladmin.get_changelist_instance(request` | 815: `changelist = modeladmin.get_changelist_instance(request` |
| strat | runner@875 | exceptions | 898: `exceptions = []` | 889: `exceptions = []` |
| strat | get_refs@240 | refs | 243: `refs \|= child.get_refs()` | 241: `refs = set()` |
| strat | test_serialize_class_based_validators@739 | validator | 802: `validator = deconstructible(path="django.core.validator` | 788: `validator = deconstructible(path="migrations.test_write` |
| strat | test03_delslice@171 | pl | 202: `pl, ul = self.lists_of_len(Len)` | 185: `del pl[:i]` |
| strat | test_create@33 | new_apps | 71: `apps = new_apps` | 38: `new_apps = Apps(["migrations"])` |
| strat | test_disable_delete_extra_formset_forms@1479 | formset | 1486: `formset = ChoiceFormFormset()` | 1492: `self.assertEqual(len(formset), 3)` |
| strat | label_for_field@358 | label | 374: `label = field.related_model._meta.verbose_name` | 400: `label = attr.short_description` |
| strat | test_change_list_facet_toggle@1043 | response | 1060: `response = self.client.get(reverse("admin:admin_views_w` | 1047: `response = self.client.get(admin_url)` |
| strat | test_datefieldlistfilter@435 | choice | 494: `choice = select_by(filterspec.choices(changelist), "dis` | 457: `self.assertIs(choice["selected"], True)` |
| strat | test_documented_range@29 | qs | 46: `qs = self.model.objects.filter(value__gte=max_value)` | 39: `qs = self.model.objects.filter(value__lte=min_value)` |
| strat | test_inheritance_deferred@221 | p | 238: `p = (` | 231: `p = (` |
| strat | test_datefieldlistfilter@435 | request | 442: `request = self.request_factory.get(` | 557: `filterspec = changelist.get_filters(request)[0][4]` |
| strat | test_naive_datetime@748 | obj | 753: `obj = next(serializers.deserialize("python", data)).obj` | 763: `obj = next(serializers.deserialize("xml", data)).object` |
| strat | test_csrf_token_on_404_stays_constant@1482 | response | 1488: `response = self.client.get("/does not exist/")` | 1487: `token1 = response.text` |
| strat | join@1135 | resolve_reuse | 1178: `resolve_reuse = set(reuse) \| {alias}` | 1176: `resolve_reuse = reuse` |
| strat | test_custom_pk@1145 | formset | 1329: `formset = FormSet(data, instance=owner1)` | 1223: `formset.forms[1].as_p(),` |
| strat | parse@530 | parse_until | 530: `def parse(self, parse_until=None):` | 540: `parse_until = []` |
| strat | test_multiple_choice_list_data@1368 | f | 1383: `f = SongForm(data)` | 1387: `f = SongForm(data)` |
| strat | test_dangling_references_throw_error@912 | project_state | 945: `project_state = ProjectState()` | 997: `project_state.apps` |
| strat | test_ticket_10790_1@1118 | q | 1135: `q = Tag.objects.exclude(parent__isnull=False)` | 1128: `self.assertSequenceEqual(q, [self.t2, self.t3, self.t4,` |
| strat | create_data@6126 | new_state | 6169: `project_state = new_state` | 6176: `new_state = new_state.clone()` |
| strat | test_annotation@325 | obj | 349: `obj = (` | 466: `obj = (` |
| strat | test_forms_with_multiple_choice@1090 | f | 1118: `f = SongForm({"name": "Yesterday", "composers": ["P"]},` | 1099: `str(f["composers"]),` |
| top20 | test_truncate_html_words@213 | truncator | 293: `truncator = text.Truncator(` | 285: `truncator = text.Truncator("foo</p>")` |
| top20 | test_change_view@2836 | response | 3021: `response = self.client.post(` | 2865: `response,` |
| top20 | test_one_to_one@359 | args | 409: `name, path, args, kwargs = field.deconstruct()` | 396: `self.assertEqual(args, [])` |
| top20 | test_dwithin_gis_lookup_output_with_rasters@280 | qs | 303: `qs = RasterModel.objects.filter(rast__dwithin=(rast, 40` | 333: `self.assertEqual(qs.count(), 1)` |
| top20 | test_foreign_key@226 | kwargs | 247: `name, path, args, kwargs = field.deconstruct()` | 244: `self.assertEqual(kwargs["to"].setting_name, "AUTH_USER_` |
| top20 | test_datefieldlistfilter@435 | choice | 456: `choice = select_by(filterspec.choices(changelist), "dis` | 602: `self.assertIs(choice["selected"], True)` |
| top20 | test_truncate_html_words@213 | truncator | 285: `truncator = text.Truncator("foo</p>")` | 313: `self.assertEqual(truncator.words(4, html=True), "hello ` |
| top20 | test_prepopulated_fields@6347 | slug2 | 6397: `slug2 = self.selenium.find_element(` | 6592: `slug2="option-two-and-now-with-a-tabular-inline",` |
| top20 | create_big_data@6199 | new_state | 6246: `new_state = new_state.clone()` | 6256: `fill_initial_data.state_forwards("fill_initial_data", n` |
| top20 | test_view_decorator@2921 | response | 2943: `response = default_view(request, "2")` | 2972: `response = other_with_prefix_view(request, "9")` |

### mypy 5ef09027: 40 sampled removed edges (seed 630; 30 stratified + 10 from top-20)
All 40 judged RIGHT (reading the source: dominating write on every path, or write-only use line, or alias twin before the real write). Loop cases were hand-read (nested loops re-enter at block top).

| kind | fn | path | def | use |
|---|---|---|---|---|
| strat | infer_condition_value@108 | result | 163: `result = MYPY_TRUE` | 155: `if result == TRUTH_VALUE_UNKNOWN:` |
| strat | partition_by_callable@6317 | uncallables | 6367: `callables, uncallables = self.partition_by_callable(` | 6347: `uncallables: list[Type] = []` |
| strat | pretty_name@407 | n | 434: `n += id` | 417: `n += "*"` |
| strat | add_args_to_env@206 | rtype | 218: `rtype = builder.type_to_rtype(arg.variable.type)` | 229: `rtype = builder.type_to_rtype(arg.variable.type)` |
| strat | run_analysis@526 | succ_map | 598: `succ_map = cfg.succ` | 595: `succ_map = cfg.pred` |
| strat | read_expression@1292 | expr | 1488: `expr = YieldExpr(value)` | 1572: `return expr` |
| strat | analyze_class_attribute_access@1132 | t | 1292: `t = add_class_tvars(` | 1291: `t = check_self_arg(t, mx.self_type, False, mx.context, ` |
| strat | visit_Delete@1261 | node | 1267: `node = DelStmt(self.visit(n.targets[0]))` | 1265: `node = DelStmt(tup)` |
| strat | check_func_def@1435 | msg | 1619: `msg = message_registry.MISSING_RETURN_STATEMENT` | 1637: `msg=msg,` |
| strat | narrow_with_len@7554 | other_types | 7572: `yes_types += other_types` | 7567: `other_types.append(t)` |
| strat | read_expression@1292 | expr | 1568: `expr = SetComprehension(generator)` | 1608: `expr = AwaitExpr(value)` |
| strat | hash_path_stem@1016 | c | 1036: `c = i64(ord(s[i]))` | 1026: `c: i64 = ord(s[i])` |
| strat | read_expression@1292 | expr | 1562: `expr = DictionaryComprehension(key, value, indices, seq` | 1434: `read_loc(data, expr)` |
| strat | visit_unbound_type_nonoptional@308 | msg | 440: `msg = f'Can\'t use bound type variable "{t.name}" to de` | 455: `msg = f'TypeVarTuple "{t.name}" is unbound'` |
| strat | check_subtype@7711 | subtype | 7739: `orig_subtype = subtype` | 7781: `call = find_member("__call__", subtype, subtype, is_ope` |
| strat | analyze_member_var_access@522 | v | 541: `v = vv.var` | 537: `vv = v` |
| strat | accept@6137 | typ2 | 6219: `typ = self.accept_maybe_cache(node, type_context=type_c` | 6200: `typ2 = node.accept(self)` |
| strat | read_expression@1292 | expr | 1575: `expr = BytesExpr(value)` | 1451: `read_loc(data, expr)` |
| strat | read_statement@312 | stmt | 545: `stmt = ImportAll(module_id, relative)` | 564: `read_loc(data, stmt)` |
| strat | update@208 | changed_modules | 296: `changed_modules = dedupe_modules(changed_modules)` | 269: `changed_modules, (next_id, next_path), blocker_messages` |
| strat | unary_not@1877 | res | 1928: `res = Register(bit_rprimitive)` | 1910: `res = Register(bit_rprimitive)` |
| strat | find_module_by_fullname@109 | head | 122: `head, tail = head.rsplit(".", maxsplit=1)` | 117: `if head in modules:` |
| strat | get_types_from_except_handler@5598 | typ | 5598: `def get_types_from_except_handler(self, typ: Type, n: E` | 5608: `merged_type = make_simplified_union((typ.args[0],))` |
| strat | narrow_type_by_identity_equality@6930 | if_type | 7136: `if_type = else_type = ambiguous_expr_type` | 7036: `if_type, else_type = conditional_types(` |
| strat | read_expression@1292 | expr | 1608: `expr = AwaitExpr(value)` | 1583: `expr = AssignmentExpr(target, value)` |
| strat | remove_misplaced_type_comments@184 | text | 198: `text = re.sub(r'^[ \t]*# +type: +["\'a-zA-Z_].*$', "", ` | 206: `text = re.sub(r"^[ \t]*# +type: +\([^()]+(\)[ \t]*)?$",` |
| strat | check_argument_types@2643 | callee_arg_kinds | 2692: `callee_arg_kinds = [nodes.ARG_STAR] + [nodes.ARG_POS] *` | 2728: `callee_arg_kinds = [ARG_POS] * len(actuals)` |
| strat | _find_module_non_stub_helper@346 | dir_path | 350: `dir_path = pkg_dir` | 361: `if not self.fscache.isdir(dir_path):` |
| strat | collect_attributes@513 | context | 732: `context: Context = cls` | 741: `context = cls` |
| strat | false_only@820 | t | 820: `def false_only(t: Type) -> ProperType:` | 834: `elif not t.can_be_true:` |
| top20 | emit_cast@653 | check | 771: `check = "(PyTuple_Check({}))"` | 808: `check = "({} == Py_None)"` |
| top20 | visit_instance@747 | suffix | 864: `suffix = instance_arg.copy_modified(` | 938: `res.append(Constraint(template_arg, SUPERTYPE_OF, suffi` |
| top20 | read_statement@312 | stmt | 461: `stmt = Import(ids)` | 522: `read_loc(data, stmt)` |
| top20 | read_statement@312 | stmt | 461: `stmt = Import(ids)` | 584: `stmt = MatchStmt(subject, patterns, guards, bodies)` |
| top20 | _type_object_overlap@518 | right | 602: `right = right.fallback` | 498: `elif isinstance(right, TypedDictType):` |
| top20 | read_expression@1292 | expr | 1367: `expr = ListExpr(items)` | 1462: `expr = ConditionalExpr(cond, if_expr, else_expr)` |
| top20 | visit_callable_type@3951 | s | 3962: `s += ", "` | 4028: `s = f"[{', '.join(vs)}] {s}"` |
| top20 | incompatible_argument@593 | name | 678: `name = callee_name[1:-1]` | 623: `base = extract_type(name)` |
| top20 | read_statement@312 | stmt | 535: `stmt = DelStmt(expr)` | 497: `return stmt` |
| top20 | test_find_sources_exclude@264 | fscache | 365: `fscache = FakeFSCache(files)` | 323: `fscache = FakeFSCache(files)` |
