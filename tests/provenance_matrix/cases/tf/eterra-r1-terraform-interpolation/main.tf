variable "region" { type = string }
locals {
  message = "region: ${var.region}"
  output = local.message
}
