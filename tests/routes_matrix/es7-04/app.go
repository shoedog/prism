package fixture

import (
	"net/http"

	backoffice "example.com/routes/es7-04/backoffice/panels"
	frontdesk "example.com/routes/es7-04/frontdesk/panels"
)

func register(mux *http.ServeMux, desk *frontdesk.Dashboard, office *backoffice.Dashboard) {
	mux.HandleFunc("GET /desk/summary", desk.Summary)
	mux.HandleFunc("GET /office/summary", office.Summary)
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	register(mux, &frontdesk.Dashboard{}, &backoffice.Dashboard{})
	return mux
}
