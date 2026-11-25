package service

import "net/http"

type Endpoint interface {
	Status(http.ResponseWriter, *http.Request)
}

type Handler = Endpoint
