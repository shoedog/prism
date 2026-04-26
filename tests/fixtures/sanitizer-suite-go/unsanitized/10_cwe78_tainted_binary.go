package main

import (
	"os/exec"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	bin := c.Query("bin")
	_ = exec.Command(bin).Run()
}
