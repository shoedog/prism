function f(c)
  x = source()
  if c then
    x = clean()
    sink(x)
  end
  sink(x)
end
