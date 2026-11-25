variable "editions" { type = object({ archive = string, current = string }) }
locals {
  display = merge(var.editions, { current = "preview" })
}
output "labels" {
  value = [
    for pair in [local.display] : {
      past = [for archive in [pair.archive] :
        archive
      ]
      now = [for current in [pair.current] :
        current
      ]
    }
  ]
}
