function f()
  x = source()
  do
    local x = clean()
    sink(x)
  end
  sink(x)
end
