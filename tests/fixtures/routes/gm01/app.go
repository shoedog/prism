package fixture

import (
	"example.com/routes/gm01/ctrl"
	"github.com/gin-gonic/gin"
)

type AnswerAPIRouter struct {
	questionController *ctrl.QuestionController
}

func (a *AnswerAPIRouter) routes() *gin.Engine {
	r := gin.New()
	r.GET("/questions", a.questionController.List)
	return r
}

func NewApp() *gin.Engine {
	a := &AnswerAPIRouter{questionController: &ctrl.QuestionController{}}
	return a.routes()
}
