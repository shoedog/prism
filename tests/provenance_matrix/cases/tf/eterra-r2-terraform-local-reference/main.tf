variable "region" { type = string }
locals {
  chosen = var.region
}
output "selected" {
  value = local.chosen
}
