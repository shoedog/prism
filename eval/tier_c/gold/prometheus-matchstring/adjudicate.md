# Adjudication — prometheus-matchstring

repo: prometheus  sha: 505095b  symbol: MatchString
oracle_health: {"lsp": "ok", "prism": "ok"}

## Disagreement band (needs verdict — source-verify; a tool provenance tag is NOT truth)

### config/config.go:432 — UnmarshalYAML
provenance: prism
d_member: none
```
	}

	for _, sf := range c.ScrapeConfigFiles {
		if !patRulePath.MatchString(sf) {
			return fmt.Errorf("invalid scrape config file path %q", sf)
		}
	}
```
verdict: 
reason: 

### discovery/file/file.go:93 — UnmarshalYAML
provenance: prism
d_member: none
```
		return errors.New("file service discovery config must contain at least one path name")
	}
	for _, name := range c.Files {
		if !patFileSDName.MatchString(name) {
			return fmt.Errorf("path name %q is not valid for file discovery", name)
		}
	}
```
verdict: 
reason: 

### discovery/http/http.go:177 — Refresh
provenance: prism
d_member: none
```
		return nil, fmt.Errorf("server returned HTTP status %s", resp.Status)
	}

	if !matchContentType.MatchString(strings.TrimSpace(resp.Header.Get("Content-Type"))) {
		d.metrics.failuresCount.Inc()
		return nil, fmt.Errorf("unsupported content type %q", resp.Header.Get("Content-Type"))
	}
```
verdict: 
reason: 

### discovery/http/http_test.go:223 — TestContentTypeRegex
provenance: prism
d_member: none
```

	for _, test := range cases {
		t.Run(test.header, func(t *testing.T) {
			require.Equal(t, test.match, matchContentType.MatchString(test.header))
		})
	}
}
```
verdict: 
reason: 

### discovery/puppetdb/puppetdb.go:215 — refresh
provenance: prism
d_member: none
```
		return nil, fmt.Errorf("server returned HTTP status %s", resp.Status)
	}

	if ct := resp.Header.Get("Content-Type"); !matchContentType.MatchString(ct) {
		return nil, fmt.Errorf("unsupported content type %s", resp.Header.Get("Content-Type"))
	}

```
verdict: 
reason: 

### model/labels/regexp.go:140 — compileMatchStringFunction
provenance: prism
d_member: none
```
		if m.stringMatcher != nil {
			return m.stringMatcher.Matches(s)
		}
		return m.re.MatchString(s)
	}
}

```
verdict: 
reason: 

### model/labels/regexp_test.go:535 — TestStringMatcherFromRegexp_LiteralPrefix
provenance: prism
d_member: none
```
				require.Falsef(t, matcher.Matches(value), "Value: %s", value)

				// Ensure the golang regexp engine would return the same.
				require.Falsef(t, re.MatchString(value), "Value: %s", value)
			}
		})
	}
```
verdict: 
reason: 

### model/labels/regexp_test.go:610 — TestStringMatcherFromRegexp_LiteralSuffix
provenance: prism
d_member: none
```
				require.Falsef(t, matcher.Matches(value), "Value: %s", value)

				// Ensure the golang regexp engine would return the same.
				require.Falsef(t, re.MatchString(value), "Value: %s", value)
			}
		})
	}
```
verdict: 
reason: 

### model/labels/regexp_test.go:695 — TestStringMatcherFromRegexp_Quest
provenance: prism
d_member: none
```
				require.Falsef(t, matcher.Matches(value), "Value: %s", value)

				// Ensure the golang regexp engine would return the same.
				require.Falsef(t, re.MatchString(value), "Value: %s", value)
			}
		})
	}
```
verdict: 
reason: 

### model/labels/regexp_test.go:1219 — TestZeroOrOneCharacterStringMatcher
provenance: prism
d_member: none
```
		requireMatches := func(s string, expected bool) {
			t.Helper()
			require.Equal(t, expected, matcher.Matches(s))
			require.Equal(t, re.MatchString(s), matcher.Matches(s))
		}

		requireMatches("\xff", true)
```
verdict: 
reason: 

### model/relabel/relabel.go:153 — Validate
provenance: prism
d_member: none
```
			return c.NameValidationScheme.IsValidLabelName(value)
		default:
			// For legacy validation, use the legacy regex that allows $variables.
			return relabelTargetLegacy.MatchString(value)
		}
	}
	if c.Action == Replace && varInRegexTemplate(c.TargetLabel) && !isValidLabelNameWithRegexVarFn(c.TargetLabel) {
```
verdict: 
reason: 

### promql/promqltest/test.go:1105 — CheckMatch
provenance: prism
d_member: none
```
	if e.regex == nil {
		return e.message == str
	}
	return e.regex.MatchString(str)
}

func (e *expectCmd) String() string {
```
verdict: 
reason: 

### promql/promqltest/test.go:1532 — checkExpectedFailure
provenance: prism
d_member: none
```
	}

	if ev.expectedFailRegexp != nil {
		if !ev.expectedFailRegexp.MatchString(actual.Error()) {
			return fmt.Errorf("expected error matching pattern %q evaluating query %q (line %d), but got: %s", ev.expectedFailRegexp.String(), ev.expr, ev.line, actual.Error())
		}
	}
```
verdict: 
reason: 

### promql/promqltest/test.go:285 — parseLoad
provenance: prism
d_member: none
```
}

func parseLoad(lines []string, i int, startTime time.Time) (int, *loadCmd, error) {
	if !patLoad.MatchString(lines[i]) {
		return i, nil, raise(i, "invalid load command. (load[_with_nhcb] <step:duration>)")
	}
	parts := patLoad.FindStringSubmatch(lines[i])
```
verdict: 
reason: 

### promql/promqltest/test.go:1810 — runInstantQuery
provenance: prism
d_member: none
```
	// Check query returns same result in range mode,
	// by checking against the middle step.
	// Skip this check for queries containing range(), step(), start(), or end() since they would resolve differently.
	if reQueryContextFuncs.MatchString(iq.expr) {
		return nil
	}
	q, err = engine.NewRangeQuery(t.context, t.storage, nil, iq.expr, iq.evalTime.Add(-time.Minute), iq.evalTime.Add(time.Minute), time.Minute)
```
verdict: 
reason: 

### promql/query_logger_test.go:58 — TestQueryLogging
provenance: prism
d_member: none
```
		queryLogger.Insert(context.Background(), queries[i])

		have := string(fileAsBytes[start:end])
		require.True(t, regexp.MustCompile(want[i]).MatchString(have),
			"Query not written correctly: %s", queries[i])
	}

```
verdict: 
reason: 

### util/logging/dedupe.go:175 — HandleWarningHeaderWithContext
provenance: prism
d_member: none
```
		return
	}

	if deprecationRegex.MatchString(message) && len(w.logged) < maxDeprecationWarnings {
		w.logged[message] = struct{}{}
	}

```
verdict: 
reason: 

## Auto-accepted (both — LSP and prism agree; still source-verify before freezing)

- model/labels/matcher.go:117 — Matches (d_member=none)
- model/labels/regexp_test.go:317 — BenchmarkFastRegexMatcher (d_member=none)
- model/labels/regexp_test.go:1456 — BenchmarkFastRegexMatcher_ConcatenatedPattern (d_member=none)
- model/labels/regexp_test.go:145 — TestFastRegexMatcher_MatchString (d_member=none)

## Fable Review Update — 2026-07-06

- Verified non-test `MatchString` file set with `git -C ~/code/bench-repos/prometheus grep -lw MatchString -- '*.go' ':!*_test.go'`: 15 files.
- Added excluded `sites[]` real-symbol bait for the 8 previously uncovered collision files: `cmd/promtool/unittest.go::matchesRun`, `model/relabel/relabel.go::relabel`, `promql/promqltest/test.go::CheckMatch`, `promql/promqltest/test_migrate.go::processTestFileLines`, `storage/remote/azuread/azuread.go::Validate`, `template/template.go::NewTemplateExpander`, `util/httputil/cors.go::SetCORS`, `util/logging/dedupe.go::HandleWarningHeaderWithContext`.
- New `|gold|/D1` = `12/9` (`d_gold_file_size=5`). Perfect-arm dry-run: `1.0 1.0 12 9 5 0` (`file_f1 d_recall gold_size d_gold_size d_gold_file_size phantom`).
