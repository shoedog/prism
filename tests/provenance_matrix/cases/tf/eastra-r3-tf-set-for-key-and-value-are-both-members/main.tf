variable "editions" { type = set(string) }
output "pairs" {
  value = [for key, item in var.editions : [
    key,
    item
  ]]
}
