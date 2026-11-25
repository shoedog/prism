package main

import (
	"fmt"
	"reflect"
	"strings"
)

func src() string {
	return "TAINT-7f3a"
}

type truthVisit struct {
	kind    reflect.Type
	address uintptr
	length  int
}

func truthReaches(value reflect.Value) bool {
	pending := []reflect.Value{value}
	seen := make(map[truthVisit]bool)
	for len(pending) > 0 {
		last := len(pending) - 1
		value = pending[last]
		pending = pending[:last]
		switch value.Kind() {
		case reflect.Map, reflect.Slice, reflect.Pointer:
			if value.IsNil() {
				continue
			}
			key := truthVisit{kind: value.Type(), address: value.Pointer()}
			if value.Kind() == reflect.Slice {
				key.length = value.Len()
			}
			if seen[key] {
				continue
			}
			seen[key] = true
		}
		switch value.Kind() {
		case reflect.String:
			if strings.Contains(value.String(), src()) {
				return true
			}
		case reflect.Map:
			entries := value.MapRange()
			for entries.Next() {
				pending = append(pending, entries.Key(), entries.Value())
			}
		case reflect.Slice, reflect.Array:
			for i := 0; i < value.Len(); i++ {
				pending = append(pending, value.Index(i))
			}
		case reflect.Struct:
			for i := 0; i < value.NumField(); i++ {
				pending = append(pending, value.Field(i))
			}
		case reflect.Pointer, reflect.Interface:
			pending = append(pending, value.Elem())
		}
	}
	return false
}

func sink(value any) {
	if truthReaches(reflect.ValueOf(value)) {
		fmt.Println("OBSERVE reaches")
	} else {
		fmt.Println("OBSERVE not")
	}
}

func truthPrintPair(index int, result string) {
	fmt.Printf("PAIR %d %s\n", index, result)
}
