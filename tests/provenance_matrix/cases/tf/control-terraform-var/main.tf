variable "x" {
  type = string
}

output "mode" {
  value = (
    var.x
  )
}
