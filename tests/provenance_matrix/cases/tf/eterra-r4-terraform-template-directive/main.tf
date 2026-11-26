variable "enabled" { type = bool }
output "result" {
  value = "%{ if var.enabled }yes%{ else }no%{ endif }"
}
