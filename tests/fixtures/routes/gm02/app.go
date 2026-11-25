package fixture

import (
	"example.com/routes/gm02/admin"
	"example.com/routes/gm02/ctrl"
	"github.com/gin-gonic/gin"
)

type apiRouter struct {
	siteInfo      *ctrl.SiteInfoController
	adminSiteInfo *admin.SiteInfoController
}

func (a *apiRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/site", a.siteInfo.Get)
	r.GET("/admin/site", a.adminSiteInfo.Get)
	return r
}

func NewApp() *gin.Engine {
	a := &apiRouter{
		siteInfo:      &ctrl.SiteInfoController{},
		adminSiteInfo: &admin.SiteInfoController{},
	}
	return a.routes()
}
