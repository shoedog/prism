variable "enabled" { type = bool }
variable "name" { type = string }
output "result" {
  value = var.enabled ? var.name : "fixed"
}
