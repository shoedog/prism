package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	cmd := c.Query("cmd")
	_ = exec.Command("sh", "-c", cmd).Run()
}
