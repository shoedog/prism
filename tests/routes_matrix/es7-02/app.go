package fixture

import "net/http"

func ledgerWithSlash(w http.ResponseWriter, _ *http.Request) {
	_, _ = w.Write([]byte("one ledger name"))
}

func ledgerInFolder(w http.ResponseWriter, _ *http.Request) {
	_, _ = w.Write([]byte("folder and ledger name"))
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /ledgers/april%2Ffinal", ledgerWithSlash)
	mux.HandleFunc("GET /ledgers/april/final", ledgerInFolder)
	return mux
}
