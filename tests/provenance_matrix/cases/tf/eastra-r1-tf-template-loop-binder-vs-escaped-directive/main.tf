variable "editions" { type = list(string) }
output "summary" {
  value = <<-TEXT
  %%{ for bogus in var.phantom }
  %{ for name in var.editions }
  ${name}
  %{ endfor }
  TEXT
}
